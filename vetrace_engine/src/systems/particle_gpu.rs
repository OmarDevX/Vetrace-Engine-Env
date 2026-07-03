use crate::components::components::{Lerp, Particle, Transform};
use crate::ecs::Entity;
use crate::{engine::engine::Engine, Behaviour};
use bytemuck::{Pod, Zeroable};
use std::sync::mpsc::Receiver;
use wgpu::{util::DeviceExt, *};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct GpuParticle {
    position: [f32; 4],
    velocity: [f32; 4],
    lifetime: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct Params {
    dt: f32,
    count: u32,
    _pad: [u32; 2],
}

pub struct GpuParticleSystem {
    device: Device,
    queue: Queue,
    pipeline: ComputePipeline,
    bgl: BindGroupLayout,
    pending: Option<PendingReadback>,
}

struct PendingReadback {
    staging: Buffer,
    entities: Vec<Entity>,
    receiver: Receiver<Result<(), BufferAsyncError>>,
}

impl GpuParticleSystem {
    pub fn new() -> Self {
        let instance = Instance::default();
        let adapter =
        pollster::block_on(instance.request_adapter(&RequestAdapterOptions::default()))
        .expect("adapter");
        let (device, queue) =
        pollster::block_on(adapter.request_device(&DeviceDescriptor::default()))
        .expect("device");
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("particle_update"),
                                                 source: ShaderSource::Wgsl(
                                                     include_str!("../../assets/shaders/wgpu/particle.comp.wgsl").into(),
                                                 ),
        });
        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("particle_bgl"),
                                                  entries: &[
                                                      BindGroupLayoutEntry {
                                                          binding: 0,
                                                          visibility: ShaderStages::COMPUTE,
                                                          ty: BindingType::Buffer {
                                                              ty: BufferBindingType::Storage { read_only: false },
                                                              has_dynamic_offset: false,
                                                              min_binding_size: None,
                                                          },
                                                          count: None,
                                                      },
                                                      BindGroupLayoutEntry {
                                                          binding: 1,
                                                          visibility: ShaderStages::COMPUTE,
                                                          ty: BindingType::Buffer {
                                                              ty: BufferBindingType::Uniform,
                                                              has_dynamic_offset: false,
                                                              min_binding_size: None,
                                                          },
                                                          count: None,
                                                      },
                                                  ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("particle_pl"),
                                                            bind_group_layouts: &[Some(&bgl)],
                                                            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("particle_pipe"),
                                                      layout: Some(&pipeline_layout),
                                                      module: &shader,
                                                      entry_point: Some("main"),
                                                      compilation_options: Default::default(),
        
            cache: None,
        });
        Self {
            device,
            queue,
            pipeline,
            bgl,
            pending: None,
        }
    }
}

impl Default for GpuParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Behaviour for GpuParticleSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        // handle completed GPU readback from previous frame
        if let Some(pending) = self.pending.take() {
            if let Ok(Ok(())) = pending.receiver.try_recv() {
                let slice = pending.staging.slice(..);
                let mapped = slice.get_mapped_range();
                let result: Vec<GpuParticle> = bytemuck::cast_slice(&mapped).to_vec();
                drop(mapped);
                pending.staging.unmap();

                use std::collections::{HashMap, HashSet};
                let map: HashMap<Entity, GpuParticle> = pending
                .entities
                .iter()
                .enumerate()
                .map(|(i, e)| (*e, result[i]))
                .collect();
                let mut lerp_entities = HashSet::new();

                {
                    let mut q = engine.world.query3_mut::<Transform, Particle, Lerp>();
                    let mut to_remove = Vec::new();
                    for (entity, transform, particle, lerp) in q.iter_mut() {
                        let Lerp::F32(inner) = lerp else { continue };
                        if let Some(gp) = map.get(entity) {
                            transform.position[0] = gp.position[0];
                            transform.position[1] = gp.position[1];
                            transform.position[2] = gp.position[2];
                            particle.lifetime = gp.lifetime;
                        }

                        lerp_entities.insert(*entity);
                        transform.size = [inner.value(); 3];
                        if particle.initial_lifetime == 0.0 {
                            particle.initial_lifetime = 1.0_f32.max(inner.speed.recip());
                        }
                        particle.lifetime = particle.initial_lifetime * (1.0 - inner.progress);

                        if particle.lifetime <= 0.0 {
                            if particle.looping {
                                particle.lifetime = particle.initial_lifetime;
                                transform.size = [particle.start_size; 3];
                                if let Some(pos) = particle.initial_position {
                                    transform.position = pos;
                                }
                            } else {
                                to_remove.push(*entity);
                            }
                        }
                    }
                    for e in to_remove {
                        engine.world.remove::<Particle>(e);
                        engine.world.remove::<Transform>(e);
                        engine.world.delete_entity(e);
                    }
                }

                let mut q = engine.world.query2_mut::<Transform, Particle>();
                let mut to_remove = Vec::new();
                for (entity, transform, particle) in q.iter_mut() {
                    if lerp_entities.contains(entity) {
                        continue;
                    }
                    if let Some(gp) = map.get(entity) {
                        transform.position[0] = gp.position[0];
                        transform.position[1] = gp.position[1];
                        transform.position[2] = gp.position[2];
                        particle.lifetime = gp.lifetime;
                    }
                    if particle.initial_lifetime == 0.0 {
                        particle.initial_lifetime = particle.lifetime.max(0.0001);
                    }
                    let progress = 1.0 - (particle.lifetime / particle.initial_lifetime);
                    let size =
                    particle.start_size + (particle.end_size - particle.start_size) * progress;
                    transform.size = [size, size, size];
                    particle.lifetime -= delta;

                    if particle.lifetime <= 0.0 {
                        if particle.looping {
                            particle.lifetime = particle.initial_lifetime;
                            transform.size = [particle.start_size; 3];
                            if let Some(pos) = particle.initial_position {
                                transform.position = pos;
                            }
                        } else {
                            to_remove.push(*entity);
                        }
                    }
                }
                for e in to_remove {
                    engine.world.remove::<Particle>(e);
                    engine.world.remove::<Transform>(e);
                    engine.world.delete_entity(e);
                }
            } else {
                // not ready yet, keep pending and poll non-blocking
                self.pending = Some(pending);
                let _ = self.device.poll(wgpu::PollType::Poll);
                return;
            }
        }

        // gather particle data for next frame
        let mut query = engine.world.query2_mut::<Transform, Particle>();
        let mut raw: Vec<(Entity, [f32; 3], [f32; 3], f32)> = Vec::new();
        for (e, transform, particle) in query.iter_mut() {
            if particle.initial_position.is_none() {
                particle.initial_position = Some(transform.position);
            }
            raw.push((*e, transform.position, particle.velocity, particle.lifetime));
        }
        drop(query);

        let mut entities = Vec::new();
        let mut data = Vec::new();
        for (e, pos, vel, life) in &raw {
            entities.push(*e);
            data.push(GpuParticle {
                position: [pos[0], pos[1], pos[2], 0.0],
                velocity: [vel[0], vel[1], vel[2], 0.0],
                lifetime: *life,
                ..GpuParticle::default()
            });
        }
        if data.is_empty() {
            return;
        }
        let buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("particles"),
                                                    contents: bytemuck::cast_slice(&data),
                                                    usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        });
        let params = Params {
            dt: delta,
            count: data.len() as u32,
            ..Default::default()
        };
        let params_buffer = self.device.create_buffer_init(&util::BufferInitDescriptor {
            label: Some("params"),
                                                           contents: bytemuck::bytes_of(&params),
                                                           usage: BufferUsages::UNIFORM | BufferUsages::COPY_SRC,
        });
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("particle_bg"),
                                                       layout: &self.bgl,
                                                       entries: &[
                                                           BindGroupEntry {
                                                               binding: 0,
                                                               resource: buffer.as_entire_binding(),
                                                           },
                                                           BindGroupEntry {
                                                               binding: 1,
                                                               resource: params_buffer.as_entire_binding(),
                                                           },
                                                       ],
        });
        let mut encoder = self
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("particle_encode"),
        });
        {
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("particle_pass"),
                                                       ..Default::default()
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            let wg = ((data.len() as f32) / 64.0).ceil() as u32;
            cpass.dispatch_workgroups(wg, 1, 1);
        }
        let staging = self.device.create_buffer(&BufferDescriptor {
            label: Some("staging"),
                                                size: (std::mem::size_of::<GpuParticle>() * data.len()) as u64,
                                                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                                                mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&buffer, 0, &staging, 0, staging.size());
        self.queue.submit(Some(encoder.finish()));
        let slice = staging.slice(..);
        use std::sync::mpsc::sync_channel;
        let (tx, rx) = sync_channel(1);
        slice.map_async(MapMode::Read, move |r| tx.send(r).unwrap());
        let _ = self.device.poll(wgpu::PollType::Poll);
        self.pending = Some(PendingReadback {
            staging,
            entities,
            receiver: rx,
        });
    }
}
