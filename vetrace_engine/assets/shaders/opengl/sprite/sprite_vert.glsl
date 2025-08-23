#version 430
layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec2 in_uv;

uniform mat4 viewProj;

out vec2 frag_uv;
out vec3 world_pos;
out float world_w;

void main() {
    frag_uv = in_uv;
    vec4 clip_pos = viewProj * vec4(in_pos, 1.0);
    world_pos = in_pos * clip_pos.w;
    world_w = clip_pos.w;
    gl_Position = clip_pos;
}
