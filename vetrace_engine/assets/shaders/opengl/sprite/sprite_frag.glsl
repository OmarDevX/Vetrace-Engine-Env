#version 430
in vec2 frag_uv;
in vec3 world_pos;
in float world_w;

out vec4 FragColor;

uniform sampler2D spriteTex;
uniform sampler2D rayDepth;
uniform vec3 camPos;
uniform int is2D;

void main() {
    vec3 pos = world_pos / world_w;
    float spriteDepth = is2D != 0 ? -pos.z : length(pos - camPos);
    vec2 uv = gl_FragCoord.xy / textureSize(rayDepth, 0);
    float sceneDepth = texture(rayDepth, uv).r;
    if (spriteDepth > sceneDepth - 0.001) discard;
    FragColor = texture(spriteTex, frag_uv);
}
