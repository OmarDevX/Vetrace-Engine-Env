use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RayTraversalBackend {
    SoftwareBvh,
    HardwareRayQuery,
}

impl RayTraversalBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SoftwareBvh => "SoftwareBvh",
            Self::HardwareRayQuery => "HardwareRayQuery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RayQuerySupport {
    pub requested: bool,
    pub adapter_supports_feature: bool,
    pub backend_supported: bool,
    pub active_backend: RayTraversalBackend,
    pub fallback_reason: Option<String>,
}

impl RayQuerySupport {
    pub fn resolve(
        adapter_features: wgpu::Features,
        requested: bool,
        backend: wgpu::Backend,
    ) -> Self {
        let adapter_supports_feature =
            adapter_features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        // wgpu currently documents EXPERIMENTAL_RAY_QUERY as Vulkan-only.
        // Do not try DX12/Metal here until wgpu exposes/supports those paths.
        let backend_supported = matches!(backend, wgpu::Backend::Vulkan);
        let (active_backend, fallback_reason) = if !requested {
            (
                RayTraversalBackend::SoftwareBvh,
                Some("VETRACE_HW_RAY_QUERY is not set to 1".to_string()),
            )
        } else if !adapter_supports_feature {
            (
                RayTraversalBackend::SoftwareBvh,
                Some("adapter does not expose Features::EXPERIMENTAL_RAY_QUERY".to_string()),
            )
        } else if !backend_supported {
            (
                RayTraversalBackend::SoftwareBvh,
                Some(format!(
                    "wgpu backend {:?} is not enabled for hardware ray query",
                    backend
                )),
            )
        } else {
            (RayTraversalBackend::HardwareRayQuery, None)
        };
        Self {
            requested,
            adapter_supports_feature,
            backend_supported,
            active_backend,
            fallback_reason,
        }
    }

    pub const fn uses_hardware(&self) -> bool {
        matches!(self.active_backend, RayTraversalBackend::HardwareRayQuery)
    }

    pub fn required_features(&self) -> wgpu::Features {
        if self.uses_hardware() {
            wgpu::Features::EXPERIMENTAL_RAY_QUERY
        } else {
            wgpu::Features::empty()
        }
    }
}

pub const RAY_QUERY_SHADOW_CASTER_MASK: u8 = 0x01;
pub const RAY_INSTANCE_METADATA_SIZE: usize = std::mem::size_of::<RayInstanceMetadata>();

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, PartialEq, Eq)]
pub struct RayInstanceMetadata {
    pub object_id: u32,
    pub material_table_offset: u32,
    pub submesh_table_offset: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RayQueryDirtyFlags {
    pub blas: bool,
    pub tlas: bool,
    pub metadata: bool,
}

#[derive(Debug)]
pub enum RayQueryAsBuildError {
    FeatureNotEnabled,
    CustomDataOutOfRange { index: u32 },
    MissingLocalMeshBuffers,
    WgpuValidation(String),
}

#[derive(Debug)]
pub struct HardwareBlasEntry {
    pub blas: wgpu::Blas,
    pub size_descriptor: wgpu::BlasTriangleGeometrySizeDescriptor,
    pub vertex_count: u32,
    pub index_count: u32,
    pub dirty: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HardwareRayQueryStats {
    pub blas_count: u32,
    pub tlas_instance_count: u32,
    pub metadata_count: u32,
    pub blas_dirty: bool,
    pub tlas_dirty: bool,
}

#[derive(Debug)]
pub struct HardwareRayQueryScene {
    pub blas_entries: Vec<HardwareBlasEntry>,
    pub tlas: Option<wgpu::Tlas>,
    pub instances: Vec<Option<wgpu::TlasInstance>>,
    pub metadata: Vec<RayInstanceMetadata>,
    pub metadata_buffer: wgpu::Buffer,
    pub tlas_capacity: u32,
    pub dirty: RayQueryDirtyFlags,
    pub last_failure: Option<String>,
}

impl HardwareRayQueryScene {
    pub fn new(device: &wgpu::Device) -> Self {
        let metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ray_query_instance_metadata_empty"),
            contents: bytemuck::bytes_of(&RayInstanceMetadata::default()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            blas_entries: Vec::new(),
            tlas: None,
            instances: Vec::new(),
            metadata: Vec::new(),
            metadata_buffer,
            tlas_capacity: 0,
            dirty: RayQueryDirtyFlags {
                blas: true,
                tlas: true,
                metadata: true,
            },
            last_failure: None,
        }
    }

    pub fn validate_custom_data(index: u32) -> Result<u32, RayQueryAsBuildError> {
        if index < (1 << 24) {
            Ok(index)
        } else {
            Err(RayQueryAsBuildError::CustomDataOutOfRange { index })
        }
    }

    fn mesh_triangle_size_descriptor(
        mesh: &crate::gpu::GpuMesh,
    ) -> wgpu::BlasTriangleGeometrySizeDescriptor {
        wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count: mesh.vertex_count,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(mesh.index_count),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        }
    }

    pub fn create_mesh_blas(
        &mut self,
        device: &wgpu::Device,
        mesh: &crate::gpu::GpuMesh,
    ) -> Result<usize, RayQueryAsBuildError> {
        if !device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err(RayQueryAsBuildError::FeatureNotEnabled);
        }
        if mesh.vertex_count == 0 || mesh.index_count == 0 {
            return Err(RayQueryAsBuildError::MissingLocalMeshBuffers);
        }

        let size_descriptor = Self::mesh_triangle_size_descriptor(mesh);
        let blas = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some(&format!("blas:{}", mesh.name)),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![Self::mesh_triangle_size_descriptor(mesh)],
            },
        );

        let index = self.blas_entries.len();
        self.blas_entries.push(HardwareBlasEntry {
            blas,
            size_descriptor,
            vertex_count: mesh.vertex_count,
            index_count: mesh.index_count,
            dirty: true,
        });
        self.dirty.blas = true;
        self.dirty.tlas = true;
        Ok(index)
    }

    pub fn mesh_blas_build_entry<'a>(
        &'a self,
        blas_index: usize,
        mesh: &'a crate::gpu::GpuMesh,
    ) -> Option<wgpu::BlasBuildEntry<'a>> {
        let entry = self.blas_entries.get(blas_index)?;
        if entry.vertex_count != mesh.vertex_count || entry.index_count != mesh.index_count {
            return None;
        }
        Some(wgpu::BlasBuildEntry {
            blas: &entry.blas,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                wgpu::BlasTriangleGeometry {
                    size: &entry.size_descriptor,
                    vertex_buffer: &mesh.vbuf,
                    first_vertex: 0,
                    vertex_stride: std::mem::size_of::<crate::gpu::Vertex>()
                        as wgpu::BufferAddress,
                    index_buffer: Some(&mesh.ibuf),
                    first_index: Some(0),
                    transform_buffer: None,
                    transform_buffer_offset: None,
                },
            ]),
        })
    }

    pub fn create_or_resize_tlas(&mut self, device: &wgpu::Device, requested_instances: u32) {
        let requested_instances = requested_instances.max(1);
        if self.tlas.is_some() && self.tlas_capacity >= requested_instances {
            return;
        }
        self.tlas = Some(device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: Some("vetrace_hardware_ray_query_tlas"),
            max_instances: requested_instances,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
        }));
        self.tlas_capacity = requested_instances;
        self.instances.clear();
        self.dirty.tlas = true;
    }

    pub fn set_tlas_instances(
        &mut self,
        device: &wgpu::Device,
        instances: Vec<Option<wgpu::TlasInstance>>,
        metadata: Vec<RayInstanceMetadata>,
    ) -> Result<(), RayQueryAsBuildError> {
        for (index, _) in metadata.iter().enumerate() {
            Self::validate_custom_data(index as u32)?;
        }
        self.create_or_resize_tlas(device, instances.len() as u32);
        if let Some(tlas) = self.tlas.as_mut() {
            if let Some(dst) = tlas.get_mut_slice(0..instances.len()) {
                dst.clone_from_slice(&instances);
            } else {
                return Err(RayQueryAsBuildError::WgpuValidation(format!(
                    "TLAS instance slice 0..{} is out of bounds for capacity {}",
                    instances.len(),
                    self.tlas_capacity
                )));
            }
        }
        self.instances = instances;
        self.metadata = metadata;
        self.dirty.tlas = true;
        self.dirty.metadata = true;
        Ok(())
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Option<wgpu::BindGroup> {
        let tlas = self.tlas.as_ref()?;
        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hardware_ray_query_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tlas.as_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.metadata_buffer.as_entire_binding(),
                },
            ],
        }))
    }

    pub fn mark_tlas_dirty_for_object_state_change(&mut self) {
        self.dirty.tlas = true;
    }
    pub fn mark_blas_dirty_for_mesh_change(&mut self) {
        self.dirty.blas = true;
        self.dirty.tlas = true;
    }

    pub fn write_metadata(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let default_metadata;
        let bytes = if self.metadata.is_empty() {
            default_metadata = RayInstanceMetadata::default();
            bytemuck::bytes_of(&default_metadata)
        } else {
            bytemuck::cast_slice(&self.metadata)
        };
        if self.metadata_buffer.size() < bytes.len() as u64 {
            self.metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ray_query_instance_metadata"),
                contents: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        } else {
            queue.write_buffer(&self.metadata_buffer, 0, bytes);
        }
        self.dirty.metadata = false;
    }

    pub fn stats(&self) -> HardwareRayQueryStats {
        HardwareRayQueryStats {
            blas_count: self.blas_entries.len() as u32,
            tlas_instance_count: self.instances.iter().filter(|i| i.is_some()).count() as u32,
            metadata_count: self.metadata.len() as u32,
            blas_dirty: self.dirty.blas,
            tlas_dirty: self.dirty.tlas,
        }
    }
}

pub fn create_hardware_ray_query_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("hardware_ray_query_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::AccelerationStructure {
                    vertex_return: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(RAY_INSTANCE_METADATA_SIZE as u64),
                },
                count: None,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ray_instance_metadata_layout_matches_wgsl() {
        assert_eq!(std::mem::size_of::<RayInstanceMetadata>(), 16);
        assert_eq!(std::mem::align_of::<RayInstanceMetadata>(), 4);
    }
    #[test]
    fn custom_data_is_limited_to_24_bits() {
        assert!(HardwareRayQueryScene::validate_custom_data(0x00ff_ffff).is_ok());
        assert!(matches!(
            HardwareRayQueryScene::validate_custom_data(0x0100_0000),
            Err(RayQueryAsBuildError::CustomDataOutOfRange { .. })
        ));
    }
}
