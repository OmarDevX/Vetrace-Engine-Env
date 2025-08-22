#version 430

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f, binding = 0) uniform image2D screen;
layout(r32f, binding = 1) uniform image2D depthTex;
layout(rgba32f, binding = 2) uniform image2D normalTex;


const int num_motion_blur_samples = 5;
const int samples_per_pixel = 4;
const int bounces = 3;
// Minimum continuation probability for Russian roulette termination
const float RR_MIN_PROB = 0.1;

struct Ray {
    vec3 Origin;
    vec3 Direction;
};

struct Object {
    vec4 orientation; // (x, y, z, w)
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
    uint _padding4;
};

struct Triangle {
    vec3 v0;
    float _padding1;
    vec3 v1;
    float _padding2;
    vec3 v2;
    float _padding3;
    vec3 n0;
    float _padding4;
    vec3 n1;
    float _padding5;
    vec3 n2;
    float _padding6;
};

struct BvhNode {
    vec4 center_radius;
    ivec4 child_object;
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
uniform float fov;
uniform int is_fisheye;
uniform vec3 skycolor;
uniform vec2 taa_jitter;
uniform float currentTime;
uniform int frameNumber;
uniform int num_objects;

layout(std430, binding = 1) buffer ObjectBuffer {
    Object objects[];
};

layout(std430, binding = 2) buffer TriangleBuffer {
    Triangle triangles[];
};

layout(std430, binding = 3) buffer BvhBuffer {
    BvhNode nodes[];
};

layout(std430, binding = 4) buffer TriBvhBuffer {
    TriBvhNode tri_nodes[];
};



float focal_length = 5.0;
float aperture = 0.01;

const float pi = 3.1415926535897932385;
const float EPSILON = 0.001;
const float MIN_HIT_T = 0.05;

uint stepRNG(uint rngState)
{
    return rngState * 747796405 + 1;
}
vec3 quat_rotate(vec4 q, vec3 v) {
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

vec4 quat_conjugate(vec4 q) {
    return vec4(-q.xyz, q.w);
}
float stepAndOutputRNGFloat(inout uint rngState)
{
    rngState  = stepRNG(rngState);
    uint word = ((rngState >> ((rngState >> 28) + 4)) ^ rngState) * 277803737;
    word      = (word >> 22) ^ word;
    return float(word) / 4294967295.0f;
}

float random(inout uint rngState)
{
    return stepAndOutputRNGFloat(rngState);
}

float van_der_corput(uint n, uint base)
{
    float inv = 1.0 / float(base);
    float denom = inv;
    float result = 0.0;
    uint i = n;
    while(i > 0u) {
        uint digit = i % base;
        result += float(digit) * denom;
        i /= base;
        denom *= inv;
    }
    return result;
}

vec2 sobol2(uint n)
{
    return vec2(van_der_corput(n, 2u), van_der_corput(n, 3u));
}

float random(inout uint rngState, float min, float max)
{
    return min + (max - min) * random(rngState);
}

vec3 random_in_unit_sphere(inout uint rngState)
{
    vec3 p = vec3(random(rngState, -1.0, 1.0), random(rngState, -1.0, 1.0), random(rngState, -1.0, 1.0));
    return normalize(p);
}

vec3 random_in_hemisphere(inout uint rngState, vec3 normal)
{
    vec3 in_unit_sphere = random_in_unit_sphere(rngState);
    if (dot(in_unit_sphere, normal) > 0.0)
        return in_unit_sphere;
    else
        return -in_unit_sphere;
}

vec3 random_cosine_direction(inout uint rngState)
{
    float r1 = random(rngState);
    float r2 = random(rngState);
    float z = sqrt(1.0 - r2);

    float phi = 2.0 * pi * r1;
    float x = cos(phi) * sqrt(r2);
    float y = sin(phi) * sqrt(r2);

    return vec3(x, y, z);
}

bool boundsIntersect(vec3 rayOrigin, vec3 rayDir, vec3 center, float radius)
{
    vec3 oc = rayOrigin - center;
    float b = dot(oc, rayDir);
    float c = dot(oc, oc) - radius * radius;
    float h = b * b - c;
    return h > 0.0;
}

bool aabbIntersect(vec3 rayOrigin, vec3 rayDir, vec3 minB, vec3 maxB, float maxDist) {
    vec3 inv = 1.0 / rayDir;
    vec3 t0 = (minB - rayOrigin) * inv;
    vec3 t1 = (maxB - rayOrigin) * inv;
    vec3 tmin = min(t0, t1);
    vec3 tmax = max(t0, t1);
    float enter = max(max(tmin.x, tmin.y), tmin.z);
    float exit = min(min(tmax.x, tmax.y), tmax.z);
    if (exit < 0.0 || enter > exit || enter > maxDist) return false;
    return true;
}

// Triangle intersection using Möller-Trumbore algorithm
bool intersectTriangle(vec3 rayOrigin, vec3 rayDir, vec3 v0, vec3 v1, vec3 v2, out float t, out vec2 uv)
{
    vec3 edge1 = v1 - v0;
    vec3 edge2 = v2 - v0;
    vec3 h = cross(rayDir, edge2);
    float a = dot(edge1, h);

    if (a > -EPSILON && a < EPSILON)
        return false; // Ray is parallel to triangle

        float f = 1.0 / a;
    vec3 s = rayOrigin - v0;
    float u = f * dot(s, h);

    if (u < 0.0 || u > 1.0)
        return false;

    vec3 q = cross(s, edge1);
    float v = f * dot(rayDir, q);

    if (v < 0.0 || u + v > 1.0)
        return false;

    t = f * dot(edge2, q);

    if (t > MIN_HIT_T)
    {
        uv = vec2(u, v);
        return true;
    }

    return false;
}

// Simplified intersection test for shadow rays. Returns true if hit within maxDist.
bool intersectTriangleShadow(vec3 rayOrigin, vec3 rayDir, vec3 v0, vec3 v1, vec3 v2, float maxDist)
{
    vec3 edge1 = v1 - v0;
    vec3 edge2 = v2 - v0;
    vec3 pvec = cross(rayDir, edge2);
    float det = dot(edge1, pvec);
    if (abs(det) < EPSILON) return false;
    float invDet = 1.0 / det;
    vec3 tvec = rayOrigin - v0;
    float u = dot(tvec, pvec) * invDet;
    if (u < 0.0 || u > 1.0) return false;
    vec3 qvec = cross(tvec, edge1);
    float v = dot(rayDir, qvec) * invDet;
    if (v < 0.0 || u + v > 1.0) return false;
    float t = dot(edge2, qvec) * invDet;
    return t > MIN_HIT_T && t < maxDist - EPSILON;
}

// Interpolate triangle normals using barycentric coordinates
vec3 interpolateNormal(vec3 n0, vec3 n1, vec3 n2, vec2 uv)
{
    float u = uv.x;
    float v = uv.y;
    float w = 1.0 - u - v;
    return normalize(w * n0 + u * n1 + v * n2);
}

float schlick(float cosine, float ref_idx)
{
    float r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0 - cosine), 5.0);
}

bool refract(vec3 v, vec3 n, float ni_over_nt, inout vec3 refracted)
{
    vec3 uv = normalize(v);
    float dt = dot(uv, n);
    float discriminant = 1.0 - ni_over_nt * ni_over_nt * (1.0 - dt * dt);
    if (discriminant > 0.0)
    {
        refracted = ni_over_nt * (uv - n * dt) - n * sqrt(discriminant);
        return true;
    }
    else
        return false;
}

vec3 reflect(vec3 v, vec3 n)
{
    return v - 2.0 * dot(v, n) * n;
}

vec3 sampleGGX(vec3 normal, float roughness, inout uint rng)
{
    if (roughness <= 0.0)
        return normal;
    float a = roughness * roughness;
    // Use constant values to avoid random noise from roughness
    float r1 = 0.5;
    float r2 = 0.5;
    float phi = 2.0 * pi * r1;
    float cosPhi = cos(phi);
    float sinPhi = sin(phi);
    float cosTheta = sqrt((1.0 - r2) / (1.0 + (a*a - 1.0) * r2));
    float sinTheta = sqrt(max(0.0, 1.0 - cosTheta*cosTheta));
    vec3 hLocal = vec3(cosPhi*sinTheta, sinPhi*sinTheta, cosTheta);
    vec3 up = abs(normal.z) < 0.999 ? vec3(0.0,0.0,1.0) : vec3(1.0,0.0,0.0);
    vec3 tangentX = normalize(cross(up, normal));
    vec3 tangentY = cross(normal, tangentX);
    vec3 h = normalize(tangentX*hLocal.x + tangentY*hLocal.y + normal*hLocal.z);
    return h;
}

void applyBloom(inout vec3 color, vec3 light, float threshold, float intensity)
{
    vec3 bloomColor = max(vec3(0.0), light - threshold);
    bloomColor *= intensity;
    color += bloomColor;
}

bool is_visible(vec3 from, vec3 to, int skipA, int skipB)
{
    vec3 dir = to - from;
    float dist = length(dir);
    dir /= dist;
    int stack[64];
    int sp = 0;
    stack[sp++] = 0;

    while (sp > 0) {
        int node_idx = stack[--sp];
        BvhNode node = nodes[node_idx];
        if (!boundsIntersect(from, dir, node.center_radius.xyz, node.center_radius.w))
            continue;
        vec3 toCenter = node.center_radius.xyz - from;
        float proj = dot(toCenter, dir);
        if (proj < 0.0 || proj - node.center_radius.w > dist) continue;
        float perp2 = dot(toCenter,toCenter) - proj*proj;
        if (perp2 > node.center_radius.w*node.center_radius.w) continue;

        int left = node.child_object.x;
        int right = node.child_object.y;
        int obj = node.child_object.z;

        if (obj >= 0) {
            if (obj == skipA || obj == skipB)
                continue;

            if (objects[obj].is_mesh > 0u) {
                if (!boundsIntersect(from, dir, objects[obj].position, length(objects[obj].size) * 0.5))
                    continue;
                int bvh_start = int(objects[obj].tri_bvh_start);
                int stack2[64];
                int sp2 = 0;
                int tests = 0;
                stack2[sp2++] = bvh_start;
                while (sp2 > 0) {
                    int n = stack2[--sp2];
                    TriBvhNode tnode = tri_nodes[n];
                    if (!aabbIntersect(from, dir, tnode.bmin.xyz, tnode.bmax.xyz, dist)) continue;
                    int l = tnode.child_tri.x;
                    int r = tnode.child_tri.y;
                    int tri = tnode.child_tri.z;
                    if (tri >= 0) {
                        vec3 v0 = triangles[tri + objects[obj].triangle_start_idx].v0 + objects[obj].position;
                        vec3 v1 = triangles[tri + objects[obj].triangle_start_idx].v1 + objects[obj].position;
                        vec3 v2 = triangles[tri + objects[obj].triangle_start_idx].v2 + objects[obj].position;
                        vec3 nrm = normalize(cross(v1 - v0, v2 - v0));
                        if (dot(dir, nrm) > 0.0) continue;
                        if (intersectTriangleShadow(from, dir, v0, v1, v2, dist))
                            return false;
                        if (++tests > 128) break;
                    } else {
                        if (l >= 0 && sp2 < 64) stack2[sp2++] = l;
                        if (r >= 0 && sp2 < 64) stack2[sp2++] = r;
                    }
                }
            } else if (objects[obj].is_cube > 0u) {
                vec3 cube_position = objects[obj].position;
                vec3 cube_size = objects[obj].size;
                vec4 cube_orientation = objects[obj].orientation;
                vec4 inv_q = quat_conjugate(cube_orientation);
                vec3 local_origin = quat_rotate(inv_q, from - cube_position);
                vec3 local_dir = quat_rotate(inv_q, dir);
                vec3 inv_dir = 1.0 / local_dir;
                vec3 cube_min = -cube_size * 0.5;
                vec3 cube_max = cube_size * 0.5;
                float tMin = (cube_min.x - local_origin.x) * inv_dir.x;
                float tMax = (cube_max.x - local_origin.x) * inv_dir.x;
                if (tMin > tMax) { float temp = tMin; tMin = tMax; tMax = temp; }
                float tyMin = (cube_min.y - local_origin.y) * inv_dir.y;
                float tyMax = (cube_max.y - local_origin.y) * inv_dir.y;
                if (tyMin > tyMax) { float temp = tyMin; tyMin = tyMax; tyMax = temp; }
                if ((tMin > tyMax) || (tyMin > tMax)) {
                } else {
                    if (tyMin > tMin) tMin = tyMin;
                    if (tyMax < tMax) tMax = tyMax;
                    float tzMin = (cube_min.z - local_origin.z) * inv_dir.z;
                    float tzMax = (cube_max.z - local_origin.z) * inv_dir.z;
                    if (tzMin > tzMax) { float temp = tzMin; tzMin = tzMax; tzMax = temp; }
                    if (!((tMin > tzMax) || (tzMin > tMax))) {
                        if (tzMin > tMin) tMin = tzMin;
                        if (tzMax < tMax) tMax = tzMax;
                        if (tMin < 0) tMin = tMax;
                        if (tMin >= 0 && tMin < dist - 0.001)
                            return false;
                    }
                }
            } else {
                vec3 sphere_position = objects[obj].position;
                float sphere_radius = objects[obj].radius;
                vec3 oc = from - sphere_position;
                float b = 2.0 * dot(oc, dir);
                float c = dot(oc, oc) - sphere_radius * sphere_radius;
                float discriminant = b * b - 4.0 * c;
                if (discriminant > 0.0) {
                    float t = (-b - sqrt(discriminant)) * 0.5;
                    if (t > 0.001 && t < dist - 0.001)
                        return false;
                }
            }
        } else {
            if (left >= 0 && sp < 64) stack[sp++] = left;
            if (right >= 0 && sp < 64) stack[sp++] = right;
        }
    }

    return true;
#else
    for (int obj = 0; obj < num_objects; ++obj) {
        if (obj == skipA || obj == skipB) continue;
        if (objects[obj].is_mesh > 0u) {
            if (!boundsIntersect(from, dir, objects[obj].position, length(objects[obj].size) * 0.5))
                continue;
            for (int t = 0; t < int(objects[obj].triangle_count); ++t) {
                int tri = t + int(objects[obj].triangle_start_idx);
                vec3 v0 = triangles[tri].v0 + objects[obj].position;
                vec3 v1 = triangles[tri].v1 + objects[obj].position;
                vec3 v2 = triangles[tri].v2 + objects[obj].position;
                vec3 nrm = normalize(cross(v1 - v0, v2 - v0));
                if (dot(dir, nrm) > 0.0) continue;
                if (intersectTriangleShadow(from, dir, v0, v1, v2, dist))
                    return false;
            }
        } else if (objects[obj].is_cube > 0u) {
            vec3 cube_position = objects[obj].position;
            vec3 cube_size = objects[obj].size;
            vec4 cube_orientation = objects[obj].orientation;
            vec4 inv_q = quat_conjugate(cube_orientation);
            vec3 local_origin = quat_rotate(inv_q, from - cube_position);
            vec3 local_dir = quat_rotate(inv_q, dir);
            vec3 inv_dir = 1.0 / local_dir;
            vec3 cube_min = -cube_size * 0.5;
            vec3 cube_max = cube_size * 0.5;
            float tMin = (cube_min.x - local_origin.x) * inv_dir.x;
            float tMax = (cube_max.x - local_origin.x) * inv_dir.x;
            if (tMin > tMax) { float temp = tMin; tMin = tMax; tMax = temp; }
            float tyMin = (cube_min.y - local_origin.y) * inv_dir.y;
            float tyMax = (cube_max.y - local_origin.y) * inv_dir.y;
            if (tyMin > tyMax) { float temp = tyMin; tyMin = tyMax; tyMax = temp; }
            if ((tMin > tyMax) || (tyMin > tMax)) {
            } else {
                if (tyMin > tMin) tMin = tyMin;
                if (tyMax < tMax) tMax = tyMax;
                float tzMin = (cube_min.z - local_origin.z) * inv_dir.z;
                float tzMax = (cube_max.z - local_origin.z) * inv_dir.z;
                if (tzMin > tzMax) { float temp = tzMin; tzMin = tzMax; tzMax = temp; }
                if (!((tMin > tzMax) || (tzMin > tMax))) {
                    if (tzMin > tMin) tMin = tzMin;
                    if (tzMax < tMax) tMax = tzMax;
                    if (tMin < 0) tMin = tMax;
                    if (tMin >= 0 && tMin < dist - 0.001)
                        return false;
                }
            }
        } else {
            vec3 sphere_position = objects[obj].position;
            float sphere_radius = objects[obj].radius;
            vec3 oc = from - sphere_position;
            float b = 2.0 * dot(oc, dir);
            float c = dot(oc, oc) - sphere_radius * sphere_radius;
            float discriminant = b * b - 4.0 * c;
            if (discriminant > 0.0) {
                float t = (-b - sqrt(discriminant)) * 0.5;
                if (t > 0.001 && t < dist - 0.001)
                    return false;
            }
        }
    }
    return true;
}

vec4 calculateLightContribution(vec3 rayOrigin, vec3 rayDir, inout uint rngState, vec3 contribution, out vec3 outNormal)
{
    vec3 light = vec3(0.0);
    float firstDepth = 1e20;
    outNormal = vec3(0.0);

    int maxBounces = bounces;
    bool inside_mesh = false;
    for (int bounce = 0; bounce < bounces; ++bounce)
    {
        float closestIntersection = 9999.0;
        int closestObjectIndex = -1;
        int hitType = 0; // 0 = sphere, 1 = cube, 2 = triangle
        vec3 hitNormal = vec3(0.0);
        float objectRoughness = 0.0;

        int stack[64];
        int sp = 0;
        stack[sp++] = 0;
        while (sp > 0) {
            int node_idx = stack[--sp];
            BvhNode node = nodes[node_idx];
            if (!boundsIntersect(rayOrigin, rayDir, node.center_radius.xyz, node.center_radius.w))
                continue;
            int left = node.child_object.x;
            int right = node.child_object.y;
            int obj = node.child_object.z;
            if (obj >= 0) {
                int i = obj;
                if (objects[i].is_mesh > 0u) {
                    if (!boundsIntersect(rayOrigin, rayDir, objects[i].position, length(objects[i].size) * 0.5))
                        continue;
                    int bvh_start = int(objects[i].tri_bvh_start);
                    int st2[64];
                    int sp2 = 0;
                    int tests = 0;
                    st2[sp2++] = bvh_start;
                    while (sp2 > 0) {
                        int n = st2[--sp2];
                        TriBvhNode tnode = tri_nodes[n];
                        if (!aabbIntersect(rayOrigin, rayDir, tnode.bmin.xyz, tnode.bmax.xyz, closestIntersection)) continue;
                        int l = tnode.child_tri.x;
                        int r = tnode.child_tri.y;
                        int tri = tnode.child_tri.z;
                        if (tri >= 0) {
                            float t;
                            vec2 uv;
                            vec3 v0 = triangles[tri + objects[i].triangle_start_idx].v0 + objects[i].position;
                            vec3 v1 = triangles[tri + objects[i].triangle_start_idx].v1 + objects[i].position;
                            vec3 v2 = triangles[tri + objects[i].triangle_start_idx].v2 + objects[i].position;
                            vec3 nrm = normalize(cross(v1 - v0, v2 - v0));
                            if (dot(rayDir, nrm) > 0.0) continue;
                            if (intersectTriangle(rayOrigin, rayDir, v0, v1, v2, t, uv)) {
                                if (t < closestIntersection) {
                                    closestIntersection = t;
                                    closestObjectIndex = i;
                                    hitType = 2;
                                    hitNormal = interpolateNormal(triangles[tri + objects[i].triangle_start_idx].n0, triangles[tri + objects[i].triangle_start_idx].n1, triangles[tri + objects[i].triangle_start_idx].n2, uv);
                                    objectRoughness = objects[i].roughness;
                                }
                            }
                            if (++tests > 128) break;
                        } else {
                            if (l >= 0 && sp2 < 64) st2[sp2++] = l;
                            if (r >= 0 && sp2 < 64) st2[sp2++] = r;
                        }
                    }
                } else if (objects[i].is_cube > 0u) {
                    vec3 cube_position = objects[i].position;
                    vec3 cube_size = objects[i].size;
                    vec4 cube_orientation = objects[i].orientation;
                    vec3 local_ray_origin = quat_rotate(quat_conjugate(cube_orientation), rayOrigin - cube_position);
                    vec3 local_ray_dir = quat_rotate(quat_conjugate(cube_orientation), rayDir);
                    vec3 cube_min = -cube_size * 0.5;
                    vec3 cube_max = cube_size * 0.5;
                    float tMin = (cube_min.x - local_ray_origin.x) / local_ray_dir.x;
                    float tMax = (cube_max.x - local_ray_origin.x) / local_ray_dir.x;
                    if (tMin > tMax) { float temp = tMin; tMin = tMax; tMax = temp; }
                    float tyMin = (cube_min.y - local_ray_origin.y) / local_ray_dir.y;
                    float tyMax = (cube_max.y - local_ray_origin.y) / local_ray_dir.y;
                    if (tyMin > tyMax) { float temp = tyMin; tyMin = tyMax; tyMax = temp; }
                    if ((tMin > tyMax) || (tyMin > tMax)) { } else {
                        if (tyMin > tMin) tMin = tyMin;
                        if (tyMax < tMax) tMax = tyMax;
                        float tzMin = (cube_min.z - local_ray_origin.z) / local_ray_dir.z;
                        float tzMax = (cube_max.z - local_ray_origin.z) / local_ray_dir.z;
                        if (tzMin > tzMax) { float temp = tzMin; tzMin = tzMax; tzMax = temp; }
                        if (!((tMin > tzMax) || (tzMin > tMax))) {
                            if (tzMin > tMin) tMin = tzMin;
                            if (tzMax < tMax) tMax = tzMax;
                            if (tMin < 0) tMin = tMax;
                            if (tMin >= 0 && tMin < closestIntersection) {
                                vec3 local_hit_point = local_ray_origin + tMin * local_ray_dir;
                                vec3 normal;
                                if (abs(local_hit_point.x - cube_min.x) < 0.001) normal = vec3(-1, 0, 0);
                                else if (abs(local_hit_point.x - cube_max.x) < 0.001) normal = vec3(1, 0, 0);
                                else if (abs(local_hit_point.y - cube_min.y) < 0.001) normal = vec3(0, -1, 0);
                                else if (abs(local_hit_point.y - cube_max.y) < 0.001) normal = vec3(0, 1, 0);
                                else if (abs(local_hit_point.z - cube_min.z) < 0.001) normal = vec3(0, 0, -1);
                                else if (abs(local_hit_point.z - cube_max.z) < 0.001) normal = vec3(0, 0, 1);
                                hitNormal = normalize(quat_rotate(cube_orientation, normal));
                                closestIntersection = tMin;
                                closestObjectIndex = i;
                                hitType = 1;
                                objectRoughness = objects[i].roughness;
                            }
                        }
                    }
                } else {
                    vec3 sphere_position = objects[i].position;
                    float sphere_radius = objects[i].radius;
                    vec3 oc = rayOrigin - sphere_position;
                    float a = dot(rayDir, rayDir);
                    float b = 2.0 * dot(oc, rayDir);
                    float c = dot(oc, oc) - sphere_radius * sphere_radius;
                    float discriminant = b * b - 4.0 * a * c;
                    if (discriminant > 0.0) {
                        float temp = (-b - sqrt(discriminant)) / (2.0 * a);
                        if (temp > 0.0 && temp < closestIntersection) {
                            closestIntersection = temp;
                            closestObjectIndex = i;
                            hitType = 0;
                            vec3 hit_point = rayOrigin + rayDir * closestIntersection;
                            hitNormal = normalize(hit_point - sphere_position);
                            objectRoughness = objects[i].roughness;
                        }
                    }
                }
            } else {
                if (left >= 0) stack[sp++] = left;
                if (right >= 0) stack[sp++] = right;
            }
        }
#else
        for (int i = 0; i < num_objects; ++i) {
            if (objects[i].is_mesh > 0u) {
                if (!boundsIntersect(rayOrigin, rayDir, objects[i].position, length(objects[i].size) * 0.5))
                    continue;
                for (int t = 0; t < int(objects[i].triangle_count); ++t) {
                    int tri = t + int(objects[i].triangle_start_idx);
                    float tt;
                    vec2 uv;
                    vec3 v0 = triangles[tri].v0 + objects[i].position;
                    vec3 v1 = triangles[tri].v1 + objects[i].position;
                    vec3 v2 = triangles[tri].v2 + objects[i].position;
                    vec3 nrm = normalize(cross(v1 - v0, v2 - v0));
                    if (dot(rayDir, nrm) > 0.0) continue;
                    if (intersectTriangle(rayOrigin, rayDir, v0, v1, v2, tt, uv)) {
                        if (tt < closestIntersection) {
                            closestIntersection = tt;
                            closestObjectIndex = i;
                            hitType = 2;
                            hitNormal = interpolateNormal(triangles[tri].n0, triangles[tri].n1, triangles[tri].n2, uv);
                            objectRoughness = objects[i].roughness;
                        }
                    }
                }
            } else if (objects[i].is_cube > 0u) {
                vec3 cube_position = objects[i].position;
                vec3 cube_size = objects[i].size;
                vec4 cube_orientation = objects[i].orientation;
                vec3 local_ray_origin = quat_rotate(quat_conjugate(cube_orientation), rayOrigin - cube_position);
                vec3 local_ray_dir = quat_rotate(quat_conjugate(cube_orientation), rayDir);
                vec3 cube_min = -cube_size * 0.5;
                vec3 cube_max = cube_size * 0.5;
                float tMin = (cube_min.x - local_ray_origin.x) / local_ray_dir.x;
                float tMax = (cube_max.x - local_ray_origin.x) / local_ray_dir.x;
                if (tMin > tMax) { float temp = tMin; tMin = tMax; tMax = temp; }
                float tyMin = (cube_min.y - local_ray_origin.y) / local_ray_dir.y;
                float tyMax = (cube_max.y - local_ray_origin.y) / local_ray_dir.y;
                if (tyMin > tyMax) { float temp = tyMin; tyMin = tyMax; tyMax = temp; }
                if ((tMin > tyMax) || (tyMin > tMax)) { } else {
                    if (tyMin > tMin) tMin = tyMin;
                    if (tyMax < tMax) tMax = tyMax;
                    float tzMin = (cube_min.z - local_ray_origin.z) / local_ray_dir.z;
                    float tzMax = (cube_max.z - local_ray_origin.z) / local_ray_dir.z;
                    if (tzMin > tzMax) { float temp = tzMin; tzMin = tzMax; tzMax = temp; }
                    if (!((tMin > tzMax) || (tzMin > tMax))) {
                        if (tzMin > tMin) tMin = tzMin;
                        if (tzMax < tMax) tMax = tzMax;
                        if (tMin < 0) tMin = tMax;
                        if (tMin >= 0 && tMin < closestIntersection) {
                            vec3 local_hit_point = local_ray_origin + tMin * local_ray_dir;
                            vec3 normal;
                            if (abs(local_hit_point.x - cube_min.x) < 0.001) normal = vec3(-1, 0, 0);
                            else if (abs(local_hit_point.x - cube_max.x) < 0.001) normal = vec3(1, 0, 0);
                            else if (abs(local_hit_point.y - cube_min.y) < 0.001) normal = vec3(0, -1, 0);
                            else if (abs(local_hit_point.y - cube_max.y) < 0.001) normal = vec3(0, 1, 0);
                            else if (abs(local_hit_point.z - cube_min.z) < 0.001) normal = vec3(0, 0, -1);
                            else if (abs(local_hit_point.z - cube_max.z) < 0.001) normal = vec3(0, 0, 1);
                            hitNormal = normalize(quat_rotate(cube_orientation, normal));
                            closestIntersection = tMin;
                            closestObjectIndex = i;
                            hitType = 1;
                            objectRoughness = objects[i].roughness;
                        }
                    }
                }
            } else {
                vec3 sphere_position = objects[i].position;
                float sphere_radius = objects[i].radius;
                vec3 oc = rayOrigin - sphere_position;
                float a = dot(rayDir, rayDir);
                float b = 2.0 * dot(oc, rayDir);
                float c = dot(oc, oc) - sphere_radius * sphere_radius;
                float discriminant = b * b - 4.0 * a * c;
                if (discriminant > 0.0) {
                    float temp = (-b - sqrt(discriminant)) / (2.0 * a);
                    if (temp > 0.0 && temp < closestIntersection) {
                        closestIntersection = temp;
                        closestObjectIndex = i;
                        hitType = 0;
                        vec3 hit_point = rayOrigin + rayDir * closestIntersection;
                        hitNormal = normalize(hit_point - sphere_position);
                        objectRoughness = objects[i].roughness;
                    }
                }
            }
        }

        if (closestObjectIndex != -1)
        {
            if (bounce == 0) {
                firstDepth = closestIntersection;
                outNormal = hitNormal;
                if (closestIntersection < 0.1) {
                    inside_mesh = true;
                    maxBounces = 1;
                }
            }
            vec3 hit_point = rayOrigin + rayDir * closestIntersection;
            vec3 normal = hitNormal;
            vec3 albedo = objects[closestObjectIndex].color/255.0;
            float emission = objects[closestObjectIndex].emission;

            float roughness = objects[closestObjectIndex].roughness;
            vec3 h = sampleGGX(normal, roughness, rngState);
            vec3 scatter_direction = normalize(reflect(rayDir, h));

            if (bounce == 0) {
                for (int j = 0; j < num_objects; ++j) {
                    if (j == closestObjectIndex) continue;
                    if (objects[j].emission > 0.0) {
                        vec3 light_pos = objects[j].position;
                        if (is_visible(hit_point + normal * 0.01, light_pos, closestObjectIndex, j)) {
                            vec3 L = normalize(light_pos - hit_point);
                            float dist2 = dot(light_pos - hit_point, light_pos - hit_point);
                            float ndotl = max(0.0, dot(normal, L));
                            vec3 emit_col = (objects[j].color / 255.0) * objects[j].emission;
                            light += contribution * emit_col * ndotl / max(dist2, 1.0);
                        }
                    }
                }
            }

            rayOrigin = hit_point + normal * (0.01 + 0.01 * random(rngState));
            rayDir = scatter_direction;

            contribution *= albedo;
            light += emission * contribution;

            // Terminate when contribution is tiny
            if (length(contribution) < 0.001)
                break;

            if (bounce >= maxBounces - 1)
                break;

            // Russian roulette termination weighted by roughness
            if (bounce > 0) {
                float rr_prob = max(RR_MIN_PROB, 1.0 - objectRoughness);
                if (random(rngState) > rr_prob)
                    break;
                contribution /= rr_prob;
            }
        }
        else
        {
            if (bounce == 0) {
                firstDepth = 1e20;
                outNormal = vec3(0.0,0.0,1.0);
            }
            vec3 skyColor = skycolor;
            light += skyColor * contribution;
            break;
        }
    }

    return vec4(light, firstDepth);
}

void main()
{
    ivec2 texel_coords = ivec2(gl_GlobalInvocationID.xy);
    ivec2 dimensions = imageSize(screen);

    if (texel_coords.x >= dimensions.x || texel_coords.y >= dimensions.y)
        return;

    uint rngState = uint(texel_coords.x + dimensions.x * texel_coords.y +
    (frameNumber + uint(currentTime * 1000.0)) * dimensions.x * dimensions.y);

    vec3 result = vec3(0.0);
    float minDepth = 1e20;
    vec3 firstNormalSum = vec3(0.0);

    for (int s = 0; s < num_motion_blur_samples; s++) {
        float time = currentTime;
        vec2 mb_jitter = vec2(float(s)) / float(num_motion_blur_samples);

        for (int i = 0; i < samples_per_pixel; i++) {
            vec2 sob = sobol2(uint(i + s * samples_per_pixel + frameNumber * samples_per_pixel * num_motion_blur_samples));
            vec2 pixel_offset = mb_jitter - 0.5 + taa_jitter + (sob - 0.5) / float(samples_per_pixel);
            vec2 pixel_center = vec2(texel_coords) + vec2(0.5) + pixel_offset;
            vec2 uv = (pixel_center / vec2(dimensions.x, dimensions.y)) * 2.0 - 1.0;
            uv.x *= float(dimensions.x) / float(dimensions.y);

            Ray ray;
            ray.Origin = camera_pos;

            if (is_fisheye != 0) {
                float r = length(uv);
                float theta = atan(r, 1.0) * 2.0;
                float phi = atan(uv.y, uv.x);

                ray.Direction = cos(theta) * camera_front +
                sin(theta) * cos(phi) * camera_right +
                sin(theta) * sin(phi) * camera_up;
            } else {
                float scale = tan(fov * 0.5);
                vec3 image_plane_dir = camera_front + uv.x * scale * camera_right + uv.y * scale * camera_up;
                ray.Direction = normalize(image_plane_dir);
            }

            vec3 contribution = vec3(1.0);
            vec3 nrm;
            vec4 res = calculateLightContribution(ray.Origin, ray.Direction, rngState, contribution, nrm);
            result += res.xyz;
            firstNormalSum += nrm;
            minDepth = min(minDepth, res.w);
        }
    }

    result = result / float(num_motion_blur_samples * samples_per_pixel);
    vec3 final_normal = normalize(firstNormalSum / float(num_motion_blur_samples * samples_per_pixel));

    vec3 final_color = clamp(result, vec3(0.0), vec3(10.0));
    imageStore(screen, texel_coords, vec4(final_color, 1.0));
    imageStore(depthTex, texel_coords, vec4(minDepth, 0.0, 0.0, 1.0));
    imageStore(normalTex, texel_coords, vec4(final_normal, 1.0));
}
