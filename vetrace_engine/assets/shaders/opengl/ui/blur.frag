#version 430
in vec2 TexCoords;
out vec4 FragColor;

uniform sampler2D screenTex;
uniform vec2 resolution;
uniform vec4 region; // x, y, width, height in pixels
uniform float feather; // transition width in pixels

const float SIGMA = 8.0;
const int RADIUS = 8;

void main() {
    vec2 tex_offset = 1.0 / resolution;
    vec4 color = vec4(0.0);
    float total = 0.0;
    for (int x = -RADIUS; x <= RADIUS; ++x) {
        for (int y = -RADIUS; y <= RADIUS; ++y) {
            vec2 offset = vec2(x, y) * tex_offset;
            float weight = exp(-(x*x + y*y) / (2.0 * SIGMA * SIGMA));
            color += texture(screenTex, TexCoords + offset) * weight;
            total += weight;
        }
    }
    vec4 blurred = color / total;
    vec4 original = texture(screenTex, TexCoords);

    vec2 frag = gl_FragCoord.xy;
    float dx = min(frag.x - region.x, region.x + region.z - frag.x);
    float dy = min(frag.y - region.y, region.y + region.w - frag.y);
    float dist = min(dx, dy);
    float alpha = clamp(dist / feather, 0.0, 1.0);

    FragColor = mix(original, blurred, alpha);
}
