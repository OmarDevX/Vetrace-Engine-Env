fn rotate_environment_direction(direction: vec3<f32>, radians: f32) -> vec3<f32> {
    let sine = sin(radians);
    let cosine = cos(radians);
    return vec3<f32>(
        direction.x * cosine - direction.z * sine,
        direction.y,
        direction.x * sine + direction.z * cosine,
    );
}

fn smooth_environment_transition(value: f32) -> f32 {
    let t = clamp(value, 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn global_environment_available() -> bool {
    return environment.params0.w >= 0.5
        && (environment.slots_counts.x != 0u || environment.slots_counts.y != 0u);
}

fn sample_environment_slot(slot: u32, direction: vec3<f32>, lod: f32) -> vec3<f32> {
    if (slot == 0u) {
        return vec3<f32>(0.0);
    }
    return textureSampleLevel(
        environment_cubemaps,
        environment_sampler,
        normalize(direction),
        i32(slot),
        clamp(lod, 0.0, f32(max(environment.slots_counts.w, 1u) - 1u)),
    ).rgb;
}

fn sample_global_environment(direction: vec3<f32>, lod: f32) -> vec3<f32> {
    if (!global_environment_available()) {
        return vec3<f32>(0.0);
    }
    let rotated = rotate_environment_direction(direction, environment.params0.z);
    let primary = sample_environment_slot(environment.slots_counts.x, rotated, lod);
    let secondary = sample_environment_slot(environment.slots_counts.y, rotated, lod);
    return mix(primary, secondary, smooth_environment_transition(environment.params0.x))
        * max(environment.params0.y, 0.0);
}

fn reflection_probe_influence(world_position: vec3<f32>, probe: ReflectionProbeGpu) -> f32 {
    let local_position = (probe.world_to_probe * vec4<f32>(world_position, 1.0)).xyz;
    let half_extents = max(probe.half_extents_blend.xyz, vec3<f32>(0.0001));
    let absolute_position = abs(local_position);
    if (any(absolute_position > half_extents)) {
        return 0.0;
    }
    let blend_distance = max(probe.half_extents_blend.w, 0.0);
    if (blend_distance <= 0.0001) {
        return 1.0;
    }
    let inner = max(half_extents - vec3<f32>(blend_distance), vec3<f32>(0.0));
    let blend_axis = max(absolute_position - inner, vec3<f32>(0.0)) / blend_distance;
    let edge = max(blend_axis.x, max(blend_axis.y, blend_axis.z));
    return 1.0 - smoothstep(0.0, 1.0, clamp(edge, 0.0, 1.0));
}

fn reflection_probe_direction(
    world_position: vec3<f32>,
    world_reflection: vec3<f32>,
    probe: ReflectionProbeGpu,
) -> vec3<f32> {
    let local_direction = normalize((probe.world_to_probe * vec4<f32>(world_reflection, 0.0)).xyz);
    if (probe.slots_modes.z == 0u) {
        return local_direction;
    }

    let local_position = (probe.world_to_probe * vec4<f32>(world_position, 1.0)).xyz;
    let half_extents = max(probe.half_extents_blend.xyz, vec3<f32>(0.0001));
    let direction_sign = select(vec3<f32>(1.0), sign(local_direction), abs(local_direction) > vec3<f32>(0.00001));
    let safe_direction = select(direction_sign * vec3<f32>(0.00001), local_direction, abs(local_direction) > vec3<f32>(0.00001));
    let t0 = (-half_extents - local_position) / safe_direction;
    let t1 = ( half_extents - local_position) / safe_direction;
    let far_t = max(t0, t1);
    let distance = min(far_t.x, min(far_t.y, far_t.z));
    if (distance <= 0.0) {
        return local_direction;
    }
    let hit_position = local_position + local_direction * distance;
    let corrected = hit_position - probe.capture_intensity.xyz;
    if (dot(corrected, corrected) <= 0.000001) {
        return local_direction;
    }
    return normalize(corrected);
}

fn sample_reflection_probe(
    world_position: vec3<f32>,
    world_reflection: vec3<f32>,
    roughness: f32,
    probe: ReflectionProbeGpu,
) -> vec3<f32> {
    let direction = reflection_probe_direction(world_position, world_reflection, probe);
    let lod = roughness * f32(max(environment.slots_counts.w, 1u) - 1u);
    let primary = sample_environment_slot(probe.slots_modes.x, direction, lod);
    let secondary = sample_environment_slot(probe.slots_modes.y, direction, lod);
    return mix(primary, secondary, smooth_environment_transition(probe.transition_params.x))
        * max(probe.capture_intensity.w, 0.0);
}

fn environment_specular_ibl(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    view_direction: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
    occlusion: f32,
) -> vec4<f32> {
    let reflection = reflect(-view_direction, normal);
    let lod = roughness * f32(max(environment.slots_counts.w, 1u) - 1u);
    let selected_count = min(u32(vetrace_custom.reflection_probe_params.x), 4u);
    var local_radiance = vec3<f32>(0.0);
    var local_weight = 0.0;
    for (var lane: u32 = 0u; lane < 4u; lane = lane + 1u) {
        if (lane >= selected_count) {
            break;
        }
        let probe_index = vetrace_custom.reflection_probe_indices[lane];
        if (probe_index >= environment.slots_counts.z) {
            continue;
        }
        let probe = reflection_probe_buffer.probes[probe_index];
        if (probe.slots_modes.x == 0u && probe.slots_modes.y == 0u) {
            continue;
        }
        let weight = reflection_probe_influence(world_position, probe);
        if (weight > 0.0) {
            local_radiance = local_radiance
                + sample_reflection_probe(world_position, reflection, roughness, probe) * weight;
            local_weight = local_weight + weight;
        }
    }

    let clamped_local_weight = clamp(local_weight, 0.0, 1.0);
    var radiance = vec3<f32>(0.0);
    if (local_weight > 1.0) {
        radiance = local_radiance / local_weight;
    } else {
        radiance = local_radiance;
    }

    let global_available = environment.params1.z >= 0.5 && global_environment_available();
    let global_weight = select(0.0, 1.0 - clamped_local_weight, global_available);
    if (global_weight > 0.0) {
        radiance = radiance + sample_global_environment(reflection, lod) * global_weight;
    }

    let brdf = textureSampleLevel(
        environment_brdf_lut,
        environment_brdf_sampler,
        vec2<f32>(max(dot(normal, view_direction), 0.0), roughness),
        0.0,
    ).rg;
    let specular = radiance * (f0 * brdf.x + vec3<f32>(brdf.y)) * occlusion;
    let coverage = clamp(clamped_local_weight + global_weight, 0.0, 1.0);
    return vec4<f32>(specular, coverage);
}

fn environment_diffuse_ibl(normal: vec3<f32>) -> vec3<f32> {
    if (environment.params1.y < 0.5) {
        return vec3<f32>(0.0);
    }
    let lod = f32(max(environment.slots_counts.w, 1u) - 1u);
    return sample_global_environment(normal, lod);
}
