#version 430

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(rgba32f, binding = 0) uniform image2D inputFrame;
layout(rgba32f, binding = 1) uniform image2D accumFrame;
layout(r32f,   binding = 2) uniform image2D depthTex;
layout(rgba32f, binding = 3) uniform image2D normalTex;

uniform vec2 taa_jitter;
uniform int frameNumber;
const float kernel[5] = float[](1.0/16.0, 4.0/16.0, 6.0/16.0, 4.0/16.0, 1.0/16.0);

void main() {
    ivec2 texel_coords = ivec2(gl_GlobalInvocationID.xy);
    ivec2 dimensions = imageSize(inputFrame);
    if (texel_coords.x >= dimensions.x || texel_coords.y >= dimensions.y)
        return;

    vec3 current = imageLoad(inputFrame, texel_coords).rgb;

    vec2 offset = taa_jitter * vec2(dimensions);
    ivec2 prev_coords = clamp(texel_coords + ivec2(offset), ivec2(0), dimensions - 1);
    vec3 prev = imageLoad(accumFrame, prev_coords).rgb;
    float centerDepth = imageLoad(depthTex, texel_coords).r;
    vec3 centerNormal = imageLoad(normalTex, texel_coords).xyz;

    vec3 final_color = current;
    if (frameNumber > 0) {
        float diff = length(prev - current);
        float history_weight = clamp(exp(-diff * 10.0), 0.1, 0.9);
        vec3 history = mix(current, prev, history_weight);

        vec3 blur = vec3(0.0);
        float total = 0.0;
        for (int dx = -2; dx <= 2; ++dx) {
            for (int dy = -2; dy <= 2; ++dy) {
                ivec2 nc = clamp(prev_coords + ivec2(dx, dy), ivec2(0), dimensions - 1);
                vec3 c = imageLoad(accumFrame, nc).rgb;
                float nd = imageLoad(depthTex, nc).r;
                vec3 nn = imageLoad(normalTex, nc).xyz;
                float w = kernel[abs(dx)] * kernel[abs(dy)];
                w *= exp(-abs(nd - centerDepth) * 40.0);
                w *= pow(max(dot(nn, centerNormal), 0.0), 8.0);
                blur += c * w;
                total += w;
            }
        }
        blur /= total;
        final_color = mix(blur, history, 0.3);
    }

    final_color = clamp(final_color, vec3(0.0), vec3(10.0));
    imageStore(accumFrame, texel_coords, vec4(final_color, 1.0));
    imageStore(inputFrame, texel_coords, vec4(final_color, 1.0));
}
