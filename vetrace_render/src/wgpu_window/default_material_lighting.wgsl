    if (index == 1) { offset = vec2<f32>(-0.840144, -0.07358); }
    if (index == 2) { offset = vec2<f32>(-0.695914,  0.457137); }
    if (index == 3) { offset = vec2<f32>(-0.203345,  0.620716); }
    if (index == 4) { offset = vec2<f32>( 0.962340, -0.194983); }
    if (index == 5) { offset = vec2<f32>( 0.473434, -0.480026); }
    if (index == 6) { offset = vec2<f32>( 0.519456,  0.767022); }
    if (index == 7) { offset = vec2<f32>( 0.185461, -0.893124); }
    if (index == 8) { offset = vec2<f32>( 0.507431,  0.064425); }
    if (index == 9) { offset = vec2<f32>( 0.896420,  0.412458); }
    if (index == 10) { offset = vec2<f32>(-0.321940, -0.932615); }
    if (index == 11) { offset = vec2<f32>(-0.791559, -0.597710); }
    return offset;
}

fn shadow_interleaved_gradient_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

fn shadow_rotate_offset(offset: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(offset.x * c - offset.y * s, offset.x * s + offset.y * c);
}

fn shadow_compare(cascade_index: i32, uv: vec2<f32>, depth: f32, offset_texels: vec2<f32>) -> f32 {
    let texel = 1.0 / max(vetrace_custom.shadow_params.y, 1.0);
    // Shadow filtering is driven by per-fragment normals, cascade selection, and
    // adaptive filter radii. Use the explicit-level comparison builtin so the
    // sample remains valid in non-uniform fragment control flow on WebGPU.
    return textureSampleCompareLevel(directional_shadow_map, directional_shadow_sampler, uv + offset_texels * texel, cascade_index, depth);
}

fn shadow_depth_load(cascade_index: i32, uv: vec2<f32>, offset_texels: vec2<i32>) -> f32 {
    let dims_u = textureDimensions(directional_shadow_map);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let safe_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let base = vec2<i32>(safe_uv * vec2<f32>(f32(dims.x), f32(dims.y)));
    let coord = clamp(base + offset_texels, vec2<i32>(0), dims - vec2<i32>(1));
    return textureLoad(directional_shadow_map, coord, cascade_index, 0);
}

fn shadow_pcf(cascade_index: i32, uv: vec2<f32>, depth: f32, radius_texels: f32) -> f32 {
    let radius = max(radius_texels, 0.0);
    if (radius < 0.25) {
        return shadow_compare(cascade_index, uv, depth, vec2<f32>(0.0));
    }

    // Rotated Poisson PCF gives a much smoother edge than a square 3x3/5x5/7x7
    // grid while using far fewer taps. This directly targets the pixelated shadow
    // edge without the heavy 49-tap receiver-side EVSM path.
    let tap_count = shadow_sample_count();
    let pixel = uv * vetrace_custom.shadow_params.y + vec2<f32>(f32(cascade_index) * 17.0, depth * 4096.0);
    let angle = shadow_interleaved_gradient_noise(pixel) * 6.28318530718;
    var sum = shadow_compare(cascade_index, uv, depth, vec2<f32>(0.0));
    var count = 1.0;
    for (var i: i32 = 0; i < 12; i = i + 1) {
        if (i < tap_count) {
            let offset = shadow_rotate_offset(shadow_poisson_offset(i), angle) * radius;
            sum = sum + shadow_compare(cascade_index, uv, depth, offset);
            count = count + 1.0;
        }
    }
    return sum / count;
}

fn average_blocker_depth(cascade_index: i32, uv: vec2<f32>, receiver_depth: f32, search_radius_texels: f32) -> vec2<f32> {
    let radius = clamp(search_radius_texels, 1.0, 5.0);
    let tap_count = min(shadow_sample_count(), 8);
    var blocker_sum = 0.0;
    var blocker_count = 0.0;

    let center_blocker = shadow_depth_load(cascade_index, uv, vec2<i32>(0, 0));
    if (center_blocker < receiver_depth) {
        blocker_sum = blocker_sum + center_blocker;
        blocker_count = blocker_count + 1.0;
    }

    for (var i: i32 = 0; i < 12; i = i + 1) {
        if (i < tap_count) {
            let offset_f = round(shadow_poisson_offset(i) * radius);
            let offset_i = vec2<i32>(i32(offset_f.x), i32(offset_f.y));
            let blocker = shadow_depth_load(cascade_index, uv, offset_i);
            if (blocker < receiver_depth) {
                blocker_sum = blocker_sum + blocker;
                blocker_count = blocker_count + 1.0;
            }
        }
    }
    if (blocker_count <= 0.0) {
        return vec2<f32>(0.0, 0.0);
    }
    return vec2<f32>(blocker_sum / blocker_count, blocker_count);
}

fn reduce_light_bleeding(p: f32, amount: f32) -> f32 {
    return clamp((p - amount) / max(1.0 - amount, 0.0001), 0.0, 1.0);
}

fn chebyshev_upper_bound(moments: vec2<f32>, receiver: f32) -> f32 {
    if (receiver <= moments.x) {
        return 1.0;
    }
    let variance = max(moments.y - moments.x * moments.x, 0.00002);
    let d = receiver - moments.x;
    return clamp(variance / (variance + d * d), 0.0, 1.0);
}

fn shadow_evsm_blurred(cascade_index: i32, uv: vec2<f32>, receiver_depth: f32, radius_texels: f32) -> f32 {
    // Full EVSM path: the depth array was converted to exponential moments and
    // blurred with horizontal + vertical passes before the scene pass. Sampling
    // here is one moment lookup plus Chebyshev evaluation, not a wide per-pixel
    // PCF loop.
    let dims_u = textureDimensions(directional_evsm_moments);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let safe_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let coord = clamp(vec2<i32>(safe_uv * vec2<f32>(f32(dims.x), f32(dims.y))), vec2<i32>(0), dims - vec2<i32>(1));
    let moments = textureLoad(directional_evsm_moments, coord, cascade_index, 0);
    let exponent = clamp(vetrace_custom.shadow_bias_extra.w, 1.0, 5.5);
    let d = clamp(receiver_depth, 0.0, 1.0);
    let positive_receiver = exp(clamp(exponent * d, -5.5, 5.5));
    let negative_receiver = -exp(clamp(-exponent * d, -5.5, 5.5));
    let positive_visibility = chebyshev_upper_bound(moments.xy, positive_receiver);
    let negative_visibility = chebyshev_upper_bound(moments.zw, negative_receiver);
    let visibility = min(positive_visibility, negative_visibility);
    return reduce_light_bleeding(visibility, 0.18);
}

fn sample_directional_shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if (vetrace_custom.shadow_params.x < 0.5) {
        return 1.0;
    }
    let cascade_index = choose_shadow_cascade(world_position);
    if (cascade_index < 0) {
        return 1.0;
    }

    let normal_bias = max(vetrace_custom.shadow_bias_extra.y, 0.0);
    let biased_position = world_position + normalize(normal) * normal_bias;
    let clip = vetrace_custom.shadow_cascade_view_proj[cascade_index] * vec4<f32>(biased_position, 1.0);
    if (abs(clip.w) < 0.00001) {
        return 1.0;
    }
    let ndc = clip.xyz / clip.w;
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0) {
        return 1.0;
    }

    let light_dir = normalize(-vetrace_custom.directional_lights[0].xyz);
    let ndotl = clamp(dot(normalize(normal), light_dir), 0.0, 1.0);
    let slope = 1.0 - ndotl;
    let base_bias = max(vetrace_custom.shadow_params.z, 0.0);
    let slope_bias = max(vetrace_custom.shadow_bias_extra.x, 0.0);
    let depth = ndc.z - base_bias * (1.0 + slope_bias * slope);
    let mode = shadow_filter_mode();

    if (mode <= 0) {
        return shadow_compare(cascade_index, uv, depth, vec2<f32>(0.0));
    }

    let soft_radius = max(vetrace_custom.shadow_params.w, 0.0);
    if (mode == 3 && cascade_index > 0) {
        return shadow_evsm_blurred(cascade_index, uv, depth, max(soft_radius, vetrace_custom.shadow_bias_extra.z));
    }

    if (soft_radius < 0.25) {
        return shadow_compare(cascade_index, uv, depth, vec2<f32>(0.0));
    }

    var filter_radius = soft_radius;
    if (mode == 2) {
        let blocker = average_blocker_depth(cascade_index, uv, depth, max(soft_radius, 1.0) * 1.5);
        if (blocker.y > 0.0) {
            let penumbra = clamp((depth - blocker.x) * vetrace_custom.shadow_params.y * max(vetrace_custom.shadow_extra.w, 0.0), soft_radius, max(soft_radius, 1.0) * 9.0);
            filter_radius = penumbra;
        }
    }
    return shadow_pcf(cascade_index, uv, depth, filter_radius);
}


fn henyey_greenstein_phase(cos_theta: f32, anisotropy: f32) -> f32 {
    let g = clamp(anisotropy, -0.95, 0.95);
