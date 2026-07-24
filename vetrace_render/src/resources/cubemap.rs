use serde::{Deserialize, Serialize};

use crate::components::CubemapHandle;
use super::assets::TextureAsset;

/// CPU-side cubemap in the canonical face order +X, -X, +Y, -Y, +Z, -Z.
///
/// `rgba8` stores base-level sRGB pixels for ordinary LDR environments.
/// `rgba16f` stores base-level linear HDR pixels as IEEE-754 half-float bits.
/// `prefiltered_rgba16f_mips` may carry an offline-generated GGX mip chain;
/// each entry is a complete six-face mip in canonical face order. The renderer
/// prefers the HDR/pre-filtered representations whenever they are present.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CubemapAsset {
    pub name: String,
    pub face_size: u32,
    #[serde(default)]
    pub rgba8: Vec<u8>,
    #[serde(default)]
    pub rgba16f: Vec<u16>,
    #[serde(default)]
    pub prefiltered_rgba16f_mips: Vec<Vec<u16>>,
    /// Increment this whenever pixel data is changed in place.
    #[serde(default)]
    pub revision: u64,
}

impl CubemapAsset {
    pub const FACE_COUNT: usize = 6;
    pub const CHANNEL_COUNT: usize = 4;

    pub fn from_faces_rgba8(
        name: impl Into<String>,
        face_size: u32,
        faces: [Vec<u8>; Self::FACE_COUNT],
    ) -> Result<Self, String> {
        validate_face_size(face_size)?;
        let expected = face_size as usize * face_size as usize * Self::CHANNEL_COUNT;
        validate_faces(&faces, expected, "RGBA8")?;
        let mut rgba8 = Vec::with_capacity(expected * Self::FACE_COUNT);
        for face in faces {
            rgba8.extend_from_slice(&face);
        }
        Ok(Self {
            name: name.into(),
            face_size,
            rgba8,
            rgba16f: Vec::new(),
            prefiltered_rgba16f_mips: Vec::new(),
            revision: 0,
        })
    }

    /// Creates a linear HDR cubemap from half-float RGBA faces. Values are the
    /// raw `f16` bit patterns so the public asset type does not expose a GPU API.
    pub fn from_faces_rgba16f(
        name: impl Into<String>,
        face_size: u32,
        faces: [Vec<u16>; Self::FACE_COUNT],
    ) -> Result<Self, String> {
        validate_face_size(face_size)?;
        let expected = face_size as usize * face_size as usize * Self::CHANNEL_COUNT;
        validate_faces(&faces, expected, "RGBA16F")?;
        let mut rgba16f = Vec::with_capacity(expected * Self::FACE_COUNT);
        for face in faces {
            rgba16f.extend_from_slice(&face);
        }
        Ok(Self {
            name: name.into(),
            face_size,
            rgba8: Vec::new(),
            rgba16f,
            prefiltered_rgba16f_mips: Vec::new(),
            revision: 0,
        })
    }

    /// Creates an HDR cubemap with a complete offline-filtered mip chain.
    /// Mip zero must be `face_size`; each following mip is half the previous
    /// size, clamped to one texel. Every mip packs all six faces.
    pub fn from_prefiltered_rgba16f_mips(
        name: impl Into<String>,
        face_size: u32,
        mips: Vec<Vec<u16>>,
    ) -> Result<Self, String> {
        validate_face_size(face_size)?;
        if mips.is_empty() {
            return Err("a prefiltered cubemap must contain at least mip zero".to_string());
        }
        let mut size = face_size;
        for (mip, pixels) in mips.iter().enumerate() {
            let expected = size as usize
                * size as usize
                * Self::CHANNEL_COUNT
                * Self::FACE_COUNT;
            if pixels.len() != expected {
                return Err(format!(
                    "prefiltered cubemap mip {mip} has {} values; expected {expected} for six {size}x{size} RGBA16F faces",
                    pixels.len()
                ));
            }
            size = (size / 2).max(1);
        }
        Ok(Self {
            name: name.into(),
            face_size,
            rgba8: Vec::new(),
            rgba16f: mips[0].clone(),
            prefiltered_rgba16f_mips: mips,
            revision: 0,
        })
    }

    /// Builds a cubemap from six already-decoded square texture assets.
    ///
    /// The required order is +X, -X, +Y, -Y, +Z, -Z. All faces must have the
    /// same dimensions and contain tightly packed RGBA8 pixels.
    pub fn from_texture_assets(
        name: impl Into<String>,
        faces: [&TextureAsset; Self::FACE_COUNT],
    ) -> Result<Self, String> {
        let face_size = faces[0].width;
        if face_size == 0 || faces[0].height != face_size {
            return Err("cubemap faces must be non-empty square RGBA8 textures".to_string());
        }
        let expected = face_size as usize * face_size as usize * Self::CHANNEL_COUNT;
        let mut packed: [Vec<u8>; Self::FACE_COUNT] = std::array::from_fn(|_| Vec::new());
        for (index, face) in faces.into_iter().enumerate() {
            if face.width != face_size || face.height != face_size {
                return Err(format!(
                    "cubemap face {index} is {}x{}; expected {face_size}x{face_size}",
                    face.width, face.height
                ));
            }
            if face.rgba8.len() != expected {
                return Err(format!(
                    "cubemap face {index} has {} bytes; expected {expected}",
                    face.rgba8.len()
                ));
            }
            packed[index] = face.rgba8.clone();
        }
        Self::from_faces_rgba8(name, face_size, packed)
    }

    pub fn expected_pixel_count(&self) -> usize {
        self.face_size as usize
            * self.face_size as usize
            * Self::CHANNEL_COUNT
            * Self::FACE_COUNT
    }

    pub fn expected_byte_len(&self) -> usize { self.expected_pixel_count() }

    pub fn is_hdr(&self) -> bool {
        self.face_size > 0 && self.rgba16f.len() == self.expected_pixel_count()
    }

    pub fn is_prefiltered(&self) -> bool {
        !self.prefiltered_rgba16f_mips.is_empty()
            && self.prefiltered_rgba16f_mips[0].len() == self.expected_pixel_count()
    }

    pub fn is_valid(&self) -> bool {
        self.face_size > 0
            && (self.rgba8.len() == self.expected_byte_len() || self.is_hdr())
    }

    pub fn face_rgba8(&self, face: usize) -> Option<&[u8]> {
        if face >= Self::FACE_COUNT || self.rgba8.len() != self.expected_byte_len() {
            return None;
        }
        let stride = self.face_size as usize * self.face_size as usize * Self::CHANNEL_COUNT;
        let start = face * stride;
        Some(&self.rgba8[start..start + stride])
    }

    pub fn face_rgba16f(&self, face: usize) -> Option<&[u16]> {
        if face >= Self::FACE_COUNT || !self.is_hdr() {
            return None;
        }
        let stride = self.face_size as usize * self.face_size as usize * Self::CHANNEL_COUNT;
        let start = face * stride;
        Some(&self.rgba16f[start..start + stride])
    }

    pub fn prefiltered_mip_rgba16f(&self, mip: usize) -> Option<&[u16]> {
        self.prefiltered_rgba16f_mips.get(mip).map(Vec::as_slice)
    }
}

fn validate_face_size(face_size: u32) -> Result<(), String> {
    if face_size == 0 {
        Err("cubemap faces must be non-empty square textures".to_string())
    } else {
        Ok(())
    }
}

fn validate_faces<T, const N: usize>(
    faces: &[Vec<T>; N],
    expected: usize,
    format: &str,
) -> Result<(), String> {
    for (index, face) in faces.iter().enumerate() {
        if face.len() != expected {
            return Err(format!(
                "cubemap face {index} has {} values; expected {expected} for the {format} format",
                face.len()
            ));
        }
    }
    Ok(())
}

impl crate::resources::RenderAssets {
    pub fn insert_cubemap(&mut self, cubemap: CubemapAsset) -> CubemapHandle {
        let mut id = self.next_cubemap;
        while self.cubemaps.contains_key(&id) {
            id = id.saturating_add(1);
        }
        self.next_cubemap = id.saturating_add(1);
        self.cubemaps.insert(id, cubemap);
        CubemapHandle(id)
    }

    /// Replaces an asset and advances its revision so the GPU pool refreshes it.
    pub fn set_cubemap(&mut self, handle: CubemapHandle, mut cubemap: CubemapAsset) {
        let next_revision = self
            .cubemaps
            .get(&handle.0)
            .map_or(cubemap.revision, |current| current.revision.saturating_add(1));
        cubemap.revision = cubemap.revision.max(next_revision);
        self.cubemaps.insert(handle.0, cubemap);
    }

    /// Marks pixels edited through `cubemaps.get_mut()` as dirty for GPU upload.
    pub fn touch_cubemap(&mut self, handle: CubemapHandle) -> bool {
        let Some(cubemap) = self.cubemaps.get_mut(&handle.0) else {
            return false;
        };
        cubemap.revision = cubemap.revision.saturating_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_face(value: u8, size: u32) -> Vec<u8> {
        vec![value; size as usize * size as usize * 4]
    }

    #[test]
    fn packs_faces_in_canonical_order() {
        let cubemap = CubemapAsset::from_faces_rgba8(
            "test",
            2,
            std::array::from_fn(|index| solid_face(index as u8, 2)),
        )
        .expect("valid faces");
        for face in 0..CubemapAsset::FACE_COUNT {
            assert_eq!(cubemap.face_rgba8(face).unwrap()[0], face as u8);
        }
    }

    #[test]
    fn supports_linear_hdr_faces() {
        let cubemap = CubemapAsset::from_faces_rgba16f(
            "hdr",
            2,
            std::array::from_fn(|index| vec![index as u16; 16]),
        )
        .expect("valid faces");
        assert!(cubemap.is_hdr());
        assert!(cubemap.rgba8.is_empty());
        assert_eq!(cubemap.face_rgba16f(5).unwrap()[0], 5);
    }

    #[test]
    fn validates_prefiltered_mip_sizes() {
        let base = vec![0_u16; 4 * 4 * 4 * CubemapAsset::FACE_COUNT];
        let mip1 = vec![0_u16; 2 * 2 * 4 * CubemapAsset::FACE_COUNT];
        let mip2 = vec![0_u16; 4 * CubemapAsset::FACE_COUNT];
        let cubemap = CubemapAsset::from_prefiltered_rgba16f_mips(
            "prefiltered",
            4,
            vec![base, mip1, mip2],
        )
        .expect("valid chain");
        assert!(cubemap.is_prefiltered());
        assert!(cubemap.is_hdr());
    }

    #[test]
    fn rejects_zero_sized_faces() {
        assert!(CubemapAsset::from_faces_rgba8(
            "empty",
            0,
            std::array::from_fn(|_| Vec::new()),
        )
        .is_err());
    }

    #[test]
    fn rejects_wrong_face_length() {
        let mut faces = std::array::from_fn(|_| solid_face(0, 2));
        faces[3].pop();
        assert!(CubemapAsset::from_faces_rgba8("bad", 2, faces).is_err());
    }

    #[test]
    fn replacing_or_touching_advances_revision() {
        let mut assets = crate::resources::RenderAssets::default();
        let original = CubemapAsset::from_faces_rgba8(
            "original",
            1,
            std::array::from_fn(|_| solid_face(0, 1)),
        )
        .unwrap();
        let handle = assets.insert_cubemap(original.clone());
        assets.set_cubemap(handle, original);
        assert_eq!(assets.cubemaps[&handle.0].revision, 1);
        assert!(assets.touch_cubemap(handle));
        assert_eq!(assets.cubemaps[&handle.0].revision, 2);
    }
}
