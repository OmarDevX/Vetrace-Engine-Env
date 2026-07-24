    let g2 = g * g;
    let denom = max(1.0 + g2 - 2.0 * g * clamp(cos_theta, -1.0, 1.0), 0.0001);
    return (1.0 - g2) / max(4.0 * PI * pow(denom, 1.5), 0.0001);
}

fn neutral_filmic(color: vec3<f32>) -> vec3<f32> {
    let x = max(color - vec3<f32>(0.004), vec3<f32>(0.0));
    return clamp((x * (6.2 * x + vec3<f32>(0.5))) / (x * (6.2 * x + vec3<f32>(1.7)) + vec3<f32>(0.06)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn reinhard_tonemap(color: vec3<f32>) -> vec3<f32> {
    return color / (vec3<f32>(1.0) + color);
}

fn apply_tonemap(color: vec3<f32>) -> vec3<f32> {
    // Internal reflection capture views write linear HDR radiance into an
    // RGBA16F cubemap. Display exposure/tone mapping is applied only when that
    // cubemap is later sampled by the visible scene.
    if (environment.params1.w >= 0.5) {
        return max(color, vec3<f32>(0.0));
    }
    let exposure = max(vetrace_custom.post_process_params.x, 0.0001);
    let gamma = clamp(vetrace_custom.post_process_params.y, 1.0, 3.0);
    let mode = i32(clamp(vetrace_custom.post_process_params.z, 0.0, 3.0));
    let exposed = max(color * exposure, vec3<f32>(0.0));
    var mapped = exposed;
    if (mode == 1) {
        mapped = aces_filmic(exposed);
    } else if (mode == 2) {
        mapped = neutral_filmic(exposed);
    } else if (mode == 3) {
        mapped = reinhard_tonemap(exposed);
    }
    // The WGPU surface is sRGB, so hardware already performs the normal
    // linear-to-sRGB conversion. Gamma 2.2 therefore means no extra shader
    // correction; other values act as an artistic display adjustment.
    let gamma_adjust = 2.2 / gamma;
    return pow(clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(gamma_adjust));
}

fn apply_simple_volumetric_fog(color: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    if (vetrace_custom.fog_params.x < 0.5 || vetrace_custom.fog_color_density.a <= 0.0) {
        return color;
    }

    let camera_to_fragment = world_position - camera.camera_position.xyz;
    let distance = length(camera_to_fragment);
    if (distance <= 0.0001) {
        return color;
    }

    let density = max(vetrace_custom.fog_color_density.a, 0.0);
    let transmittance = exp(-density * distance);
    let fog_amount = clamp(1.0 - transmittance, 0.0, 0.96);
    let ray_dir = camera_to_fragment / distance;

    var light_dir = normalize(-vetrace_custom.directional_lights[0].xyz);
    if (dot(light_dir, light_dir) < 0.0001) {
        light_dir = normalize(vec3<f32>(0.35, 1.0, 0.25));
    }
    let cos_theta = dot(ray_dir, light_dir);
    let phase = henyey_greenstein_phase(cos_theta, vetrace_custom.fog_params.y);
    let normalized_phase = clamp(phase * 4.0 * PI, 0.25, 3.0);

    let fog_color = max(vetrace_custom.fog_color_density.rgb, vec3<f32>(0.0));
    let ambient = clamp(vetrace_custom.light_counts.w, 0.0, 1.0);
    let sun_color = max(vetrace_custom.directional_colors[0].rgb, vec3<f32>(0.0)) * max(vetrace_custom.directional_lights[0].w, 0.0);
    let in_scatter = fog_color * (0.45 + ambient * 0.55) + sun_color * fog_color * normalized_phase * 0.18;

    return mix(color, in_scatter, fog_amount);
}

fn baked_probe_irradiance(normal: vec3<f32>) -> vec3<f32> {
    if (vetrace_custom.baked_gi_params.y < 0.5) {
        return vec3<f32>(0.0);
    }
    let n = normalize(normal);
    let x = n.x;
    let y = n.y;
    let z = n.z;
    let irradiance =
        vetrace_custom.baked_probe_sh0.rgb * 0.282095
        + vetrace_custom.baked_probe_sh1.rgb * (0.488603 * y)
        + vetrace_custom.baked_probe_sh2.rgb * (0.488603 * z)
        + vetrace_custom.baked_probe_sh3.rgb * (0.488603 * x)
        + vetrace_custom.baked_probe_sh4.rgb * (1.092548 * x * y)
        + vetrace_custom.baked_probe_sh5.rgb * (1.092548 * y * z)
        + vetrace_custom.baked_probe_sh6.rgb * (0.315392 * (3.0 * z * z - 1.0))
        + vetrace_custom.baked_probe_sh7.rgb * (1.092548 * x * z)
        + vetrace_custom.baked_probe_sh8.rgb * (0.546274 * (x * x - y * y));
    return max(irradiance, vec3<f32>(0.0));
}

fn baked_lightmap_irradiance(lightmap_uv: vec2<f32>) -> vec3<f32> {
    if (vetrace_custom.baked_gi_params.x < 0.5) {
        return vec3<f32>(0.0);
    }
    var uv = lightmap_uv * vetrace_custom.baked_lightmap_transform.xy + vetrace_custom.baked_lightmap_transform.zw;
    // Two-layer atlas layout: combined direct+indirect in the top half,
    // indirect-only in the bottom half for hybrid realtime direct shadows.
    let lightmap_debug = vetrace_custom.baked_gi_extra.x > 0.5
        && vetrace_custom.baked_gi_extra.x < 1.5;
    if (vetrace_custom.baked_gi_extra.y >= 0.5 && !lightmap_debug) {
        uv.y = uv.y + 0.5;
    }
    let irradiance = textureSample(baked_lightmap_texture, custom_render_texture_sampler, uv).rgb;
    return max(irradiance * vetrace_custom.baked_gi_params.w, vec3<f32>(0.0));
}

fn evaluate_pbr_light(
    n: vec3<f32>,
    v: vec3<f32>,
    l: vec3<f32>,
    radiance: vec3<f32>,
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32,
    f0: vec3<f32>
) -> vec3<f32> {
    let ndotl = max(dot(n, l), 0.0);
    if (ndotl <= 0.0) {
        return vec3<f32>(0.0);
    }
    let ndotv = max(dot(n, v), 0.0);
    let h = normalize(v + l);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let d = distribution_ggx(n, h, roughness);
    let g = geometry_smith(n, v, l, roughness);
    let specular = (d * g * f) / max(4.0 * ndotv * ndotl, 0.0001);
    let kd = (vec3<f32>(1.0) - f) * (1.0 - metallic);
    let diffuse = kd * base_color / PI;
    return (diffuse + specular) * radiance * ndotl;
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let health = clamp(vetrace_custom.time_health.y, 0.0, 1.0);
    let material_uv = input.uv * max(vetrace_custom.params[0].xy, vec2<f32>(0.0001));
    let base_tex = textureSample(base_color_texture, material_sampler, material_uv);
    let mr_tex = textureSample(metallic_roughness_texture, material_sampler, material_uv);
    let occlusion_tex = textureSample(occlusion_texture, material_sampler, material_uv).r;
    let emissive_tex = textureSample(emissive_texture, material_sampler, material_uv).rgb;

    let alpha = clamp(vetrace_custom.pbr_params.z * vetrace_custom.color_a.a * input.color.a * base_tex.a, 0.0, 1.0);
    let alpha_mode = vetrace_custom.pbr_extra.w;
    let alpha_cutoff = clamp(vetrace_custom.pbr_extra.z, 0.0, 1.0);
    if (alpha_mode > 0.5 && alpha_mode < 1.5 && alpha < alpha_cutoff) {
        discard;
    }
    if (alpha <= 0.001) {
        discard;
    }
    var output_alpha = alpha;
    if (alpha_mode > 0.5 && alpha_mode < 1.5) {
        output_alpha = 1.0;
    }
    let roughness = clamp(vetrace_custom.pbr_params.x * mr_tex.g, 0.04, 1.0);
    let metallic = clamp(vetrace_custom.pbr_params.y * mr_tex.b, 0.0, 1.0);
    let occlusion_strength = clamp(vetrace_custom.pbr_extra.y, 0.0, 1.0);
    let occlusion = mix(1.0, occlusion_tex, occlusion_strength);

    let base_color = clamp(vetrace_custom.color_a.rgb * input.color.rgb * base_tex.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * health;
    let emissive = max(vetrace_custom.color_b.rgb * emissive_tex, vec3<f32>(0.0));
    var n = normal_from_map(input);
    if (!input.front_facing) {
        n = -n;
    }
    let v = normalize(camera.camera_position.xyz - input.world_position);
    let ndotv = max(dot(n, v), 0.0);
    let ambient = clamp(vetrace_custom.light_counts.w, 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), base_color, metallic);

    let baked_debug_mode = vetrace_custom.baked_gi_extra.x;
    if (baked_debug_mode > 0.5 && baked_debug_mode < 1.5) {
        // Static receivers show their decoded lightmap. Dynamic probe-only
        // receivers have no atlas allocation, so show their probe irradiance
        // instead of a misleading solid-black object.
        if (vetrace_custom.baked_gi_params.x >= 0.5) {
            let debug_irradiance = baked_lightmap_irradiance(input.lightmap_uv);
            return vec4(apply_tonemap(debug_irradiance), 1.0);
        }
        if (vetrace_custom.baked_gi_params.y >= 0.5) {
            return vec4(apply_tonemap(baked_probe_irradiance(n)), 1.0);
        }
        return vec4(vec3<f32>(0.015), 1.0);
    }
    if (baked_debug_mode >= 1.5 && baked_debug_mode < 2.5) {
        let uv = input.lightmap_uv;
        let checker = fract(floor(uv.x * 16.0) * 0.5 + floor(uv.y * 16.0) * 0.5) * 2.0;
        return vec4(vec3<f32>(fract(uv.x), fract(uv.y), 0.25 + checker * 0.5), 1.0);
    }
    if (baked_debug_mode >= 2.5) {
        return vec4(apply_tonemap(baked_probe_irradiance(n)), 1.0);
    }

    var direct = vec3<f32>(0.0);
    for (var i: i32 = 0; i < 4; i = i + 1) {
        if (f32(i) >= vetrace_custom.light_counts.x) {
            break;
        }
        let light = vetrace_custom.directional_lights[i];
        let color = max(vetrace_custom.directional_colors[i].rgb, vec3<f32>(0.0));
        let l = normalize(-light.xyz);
        // Main-camera cascade maps are not valid for six cubemap cameras.
        // Capture direct light without reusing those view-dependent shadows.
        // Capture views receive either capture-camera shadow matrices or a
        // disabled ShadowInfo. Do not sample stale main-camera cascades.
        var shadow = 1.0;
        if (i == 0) {
            shadow = sample_directional_shadow(input.world_position, n);
        }
        let radiance = color * max(light.w, 0.0) * shadow;
        direct = direct + evaluate_pbr_light(n, v, l, radiance, base_color, metallic, roughness, f0);
    }

    for (var i: i32 = 0; i < 8; i = i + 1) {
        if (f32(i) >= vetrace_custom.light_counts.y) {
            break;
        }
        let light = vetrace_custom.point_lights[i];
        let color_range = vetrace_custom.point_colors_ranges[i];
        let to_light = light.xyz - input.world_position;
        let distance2 = max(dot(to_light, to_light), 0.0001);
        let distance = sqrt(distance2);
        let l = to_light / distance;
        let attenuation = range_attenuation(distance, color_range.w) / distance2;
        let radiance = max(color_range.rgb, vec3<f32>(0.0)) * max(light.w, 0.0) * attenuation;
        direct = direct + evaluate_pbr_light(n, v, l, radiance, base_color, metallic, roughness, f0);
    }

    for (var i: i32 = 0; i < 4; i = i + 1) {
        if (f32(i) >= vetrace_custom.light_counts.z) {
            break;
        }
        let light = vetrace_custom.spot_lights[i];
        let dir_range = vetrace_custom.spot_dirs_ranges[i];
        let color_inner = vetrace_custom.spot_colors_inner[i];
        let to_light = light.xyz - input.world_position;
        let distance2 = max(dot(to_light, to_light), 0.0001);
        let distance = sqrt(distance2);
        let l = to_light / distance;
        let light_to_fragment = -l;
        let spot_dir = normalize(dir_range.xyz);
        let theta = dot(light_to_fragment, spot_dir);
        let inner = color_inner.w;
        let outer = vetrace_custom.spot_params[i].x;
        let cone = clamp((theta - outer) / max(inner - outer, 0.001), 0.0, 1.0);
        let attenuation = range_attenuation(distance, dir_range.w) * cone * cone / distance2;
        let radiance = max(color_inner.rgb, vec3<f32>(0.0)) * max(light.w, 0.0) * attenuation;
        direct = direct + evaluate_pbr_light(n, v, l, radiance, base_color, metallic, roughness, f0);
    }

    // Baked lightmaps replace the static object's diffuse lighting. Directional
    // probes add low-cost diffuse GI to moving objects. Runtime direct lights
    // remain available unless the receiver requested static-lighting-only.
    let ambient_f = fresnel_schlick_roughness(ndotv, f0, roughness);
    let lightmap_irradiance = baked_lightmap_irradiance(input.lightmap_uv);
    let probe_irradiance = baked_probe_irradiance(n);
    // A valid probe volume replaces the generic white ambient floor. Adding
    // both washed out directional color bleed and could tone-map pale moving
    // objects to solid white even though their probe sample was correct.
    let has_baked_probes = vetrace_custom.baked_gi_params.y >= 0.5;
    let fallback_ambient = vec3<f32>(select(ambient, 0.0, has_baked_probes));
    let environment_diffuse = select(environment_diffuse_ibl(n) / PI, vec3<f32>(0.0), has_baked_probes);
    let dynamic_diffuse = base_color * (fallback_ambient + probe_irradiance / PI + environment_diffuse);
    let baked_diffuse = base_color * (lightmap_irradiance / PI);
    let ambient_diffuse = select(dynamic_diffuse, baked_diffuse, vetrace_custom.baked_gi_params.x >= 0.5) * (1.0 - metallic) * occlusion;

    let probe_specular_floor = max(max(probe_irradiance.r, probe_irradiance.g), probe_irradiance.b) * 0.12;
    let fallback_specular = select(ambient, 0.0, has_baked_probes);
    let legacy_ambient_specular = ambient_f * (fallback_specular + probe_specular_floor) * (0.55 + (1.0 - roughness) * 0.45) * occlusion;
    let cubemap_specular = environment_specular_ibl(input.world_position, n, v, roughness, f0, occlusion);
    let ambient_specular = legacy_ambient_specular * (1.0 - cubemap_specular.a) + cubemap_specular.rgb;

    let lit_color = ambient_diffuse + ambient_specular + direct + emissive;
    // Fog is a camera/view effect. Baking it into reflection captures would
    // compound fog when the reflection is later rendered through the main view.
    var color = lit_color;
    if (environment.params1.w < 0.5) {
        color = apply_simple_volumetric_fog(lit_color, input.world_position);
    }
    return vec4<f32>(apply_tonemap(color), output_alpha);
}
