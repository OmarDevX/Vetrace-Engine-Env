struct PrefilterUniform {
    face_sample_count: vec4<u32>,
    params: vec4<f32>,
};

@group(0) @binding(0)
var source_cubemap: texture_cube<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(0) @binding(2)
var<uniform> prefilter: PrefilterUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = positions[vertex_index] * 0.5 + vec2<f32>(0.5);
    return output;
}

fn radical_inverse_vdc(bits_value: u32) -> f32 {
    var bits = bits_value;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}

fn hammersley(index: u32, count: u32) -> vec2<f32> {
    return vec2<f32>(f32(index) / max(f32(count), 1.0), radical_inverse_vdc(index));
}

fn cube_direction(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let p = uv * 2.0 - vec2<f32>(1.0);
    if (face == 0u) { return normalize(vec3<f32>( 1.0, -p.y, -p.x)); }
    if (face == 1u) { return normalize(vec3<f32>(-1.0, -p.y,  p.x)); }
    if (face == 2u) { return normalize(vec3<f32>( p.x,  1.0,  p.y)); }
    if (face == 3u) { return normalize(vec3<f32>( p.x, -1.0, -p.y)); }
    if (face == 4u) { return normalize(vec3<f32>( p.x, -p.y,  1.0)); }
    return normalize(vec3<f32>(-p.x, -p.y, -1.0));
}

fn importance_sample_ggx(xi: vec2<f32>, normal: vec3<f32>, roughness: f32) -> vec3<f32> {
    let alpha = max(roughness * roughness, 0.0001);
    let phi = 6.28318530718 * xi.x;
    let cos_theta = sqrt((1.0 - xi.y) / max(1.0 + (alpha * alpha - 1.0) * xi.y, 0.0001));
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let half_tangent = vec3<f32>(cos(phi) * sin_theta, sin(phi) * sin_theta, cos_theta);

    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(normal.z) > 0.999);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    return normalize(tangent * half_tangent.x + bitangent * half_tangent.y + normal * half_tangent.z);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = cube_direction(prefilter.face_sample_count.x, input.uv);
    let roughness = clamp(prefilter.params.x, 0.0, 1.0);
    if (roughness <= 0.0001) {
        return vec4<f32>(textureSampleLevel(source_cubemap, source_sampler, normal, 0.0).rgb, 1.0);
    }

    let view_direction = normal;
    let sample_count = clamp(prefilter.face_sample_count.y, 1u, 256u);
    var color = vec3<f32>(0.0);
    var total_weight = 0.0;
    for (var index: u32 = 0u; index < 256u; index = index + 1u) {
        if (index >= sample_count) { break; }
        let half_vector = importance_sample_ggx(hammersley(index, sample_count), normal, roughness);
        let light_direction = normalize(2.0 * dot(view_direction, half_vector) * half_vector - view_direction);
        let ndotl = max(dot(normal, light_direction), 0.0);
        if (ndotl > 0.0) {
            color = color + textureSampleLevel(source_cubemap, source_sampler, light_direction, 0.0).rgb * ndotl;
            total_weight = total_weight + ndotl;
        }
    }
    return vec4<f32>(color / max(total_weight, 0.0001), 1.0);
}
