#version 430

in vec2 TexCoords;

out vec4 FragColor;

uniform sampler2D screenTex;
uniform vec2 texSize;
uniform float sharpness;

void main() {
    vec2 texel = 1.0 / texSize;
    vec3 c = texture(screenTex, TexCoords).rgb;
    vec3 n = texture(screenTex, TexCoords + vec2(texel.x, 0.0)).rgb;
    vec3 s = texture(screenTex, TexCoords - vec2(texel.x, 0.0)).rgb;
    vec3 e = texture(screenTex, TexCoords + vec2(0.0, texel.y)).rgb;
    vec3 w = texture(screenTex, TexCoords - vec2(0.0, texel.y)).rgb;
    vec3 avg = (n + s + e + w) * 0.25;
    vec3 sharpened = c + sharpness * (c - avg);
    FragColor = vec4(sharpened, 1.0);
}
