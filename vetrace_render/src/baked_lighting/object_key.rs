// Stable scene-object hashing used to reconnect baked data.

/// Stable key used to reconnect baked data after a scene is instantiated again.
/// It intentionally excludes transient ECS entity IDs and texture handles.
use super::*;

pub(crate) fn baked_object_key(object: &RenderObject, assets: Option<&RenderAssets>) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    hash_bytes(&mut hash, object.name.as_deref().unwrap_or("<unnamed>").as_bytes());
    hash_transform(&mut hash, &object.transform);
    if let Some(shape) = &object.shape {
        hash_u64(&mut hash, primitive_tag(shape.primitive));
        for value in shape.size.to_array() { hash_i64(&mut hash, quantize(value)); }
    }
    if let Some(mesh) = object.mesh {
        if let Some(asset) = assets.and_then(|assets| assets.meshes.get(&mesh.0)) {
            hash_bytes(&mut hash, asset.name.as_bytes());
            hash_u64(&mut hash, asset.vertices.len() as u64);
            hash_u64(&mut hash, asset.indices.len() as u64);
        } else {
            hash_u64(&mut hash, mesh.0);
        }
    }
    hash
}

fn primitive_tag(value: PrimitiveShape) -> u64 { match value { PrimitiveShape::Cube => 1, PrimitiveShape::Sphere => 2, PrimitiveShape::Capsule => 3, PrimitiveShape::Plane => 4, PrimitiveShape::Quad => 5 } }
fn quantize(value: f32) -> i64 { (value * 10_000.0).round() as i64 }
fn hash_transform(hash: &mut u64, transform: &GlobalTransform) {
    for value in transform.translation.to_array() { hash_i64(hash, quantize(value)); }
    for value in transform.rotation.to_array() { hash_i64(hash, quantize(value)); }
    for value in transform.scale.to_array() { hash_i64(hash, quantize(value)); }
}
fn hash_i64(hash: &mut u64, value: i64) { hash_bytes(hash, &value.to_le_bytes()); }
fn hash_u64(hash: &mut u64, value: u64) { hash_bytes(hash, &value.to_le_bytes()); }
fn hash_bytes(hash: &mut u64, bytes: &[u8]) { for byte in bytes { *hash ^= *byte as u64; *hash = hash.wrapping_mul(0x100000001b3); } }
