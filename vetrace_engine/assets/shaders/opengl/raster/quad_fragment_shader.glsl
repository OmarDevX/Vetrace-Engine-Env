#version 430

in vec2 TexCoords;

out vec4 FragColor;

uniform sampler2D screenTex;

void main() {
    FragColor = texture(screenTex, TexCoords);
}
