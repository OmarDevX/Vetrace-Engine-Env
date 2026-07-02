#version 430

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f, binding = 0) uniform image2D screen;
layout(r32f, binding = 1) uniform image2D depthTex;
layout(rgba32f, binding = 2) uniform image2D normalTex;

struct Object {
    vec4 orientation;
    vec3 position;
    float _padding1;
    vec3 size;
    float _padding2;
    vec3 color;
    float _padding3;
    float radius;
    float roughness;
    float emission;
    float refractive_index;
    uint is_cube;
    uint is_glass;
    uint is_mesh;
    uint triangle_start_idx;
    uint triangle_count;
    uint tri_bvh_start;
    uint tri_bvh_count;
    uint is_shaded;
};

struct TriBvhNode {
    vec4 bmin;
    vec4 bmax;
    ivec4 child_tri;
};

uniform vec3 camera_pos;
uniform vec3 camera_front;
uniform vec3 camera_up;
uniform vec3 camera_right;
uniform vec3 camera_velocity;
uniform float fov;
uniform int is_fisheye;
uniform vec3 skycolor;
uniform int is_accumulation;
uniform float currentTime;
uniform int frameNumber;
uniform int num_objects;

layout(std430, binding = 1) buffer ObjectBuffer {
    Object objects[];
};

void main() {
    ivec2 texel_coords = ivec2(gl_GlobalInvocationID.xy);
    ivec2 dimensions = imageSize(screen);

    if (texel_coords.x >= dimensions.x || texel_coords.y >= dimensions.y)
        return;

    vec2 pixel_pos = vec2(texel_coords);
    vec2 screen_center = vec2(dimensions) * 0.5;
    float scale = float(dimensions.y) / (fov*10);
    vec3 out_color = skycolor;
    float top_layer = -1e20;
    float depth_val = 1e20;

    for (int i = 0; i < num_objects; ++i) {
        vec2 obj_pos = (objects[i].position.xy - camera_pos.xy) * scale + screen_center;
        float layer = objects[i].position.z;
        vec3 color = objects[i].color / 255.0;
        bool inside = false;
        if (objects[i].is_cube > 0u) {
            vec2 half_size = objects[i].size.xy * 0.5 * scale;
            vec2 diff = pixel_pos - obj_pos;
            // rotate into object space using orientation around Z
            float angle = 2.0 * atan(objects[i].orientation.z, objects[i].orientation.w);
            float s = sin(-angle);
            float c = cos(-angle);
            vec2 local = vec2(diff.x * c - diff.y * s, diff.x * s + diff.y * c);
            if (abs(local.x) <= half_size.x && abs(local.y) <= half_size.y) {
                inside = true;
            }
        } else {
            float r = objects[i].radius * scale;
            if (distance(pixel_pos, obj_pos) <= r) {
                inside = true;
            }
        }
        if (inside && layer >= top_layer) {
            top_layer = layer;
            out_color = color;
            depth_val = -layer;
        }
    }

    imageStore(screen, texel_coords, vec4(out_color, 1.0));
    imageStore(depthTex, texel_coords, vec4(depth_val, 0.0, 0.0, 1.0));
    imageStore(normalTex, texel_coords, vec4(0.0, 0.0, 1.0, 1.0));
}
