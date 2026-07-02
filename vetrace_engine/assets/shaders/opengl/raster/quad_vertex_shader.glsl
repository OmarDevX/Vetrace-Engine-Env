#version 430

layout(location = 0) in vec2 in_pos;

out vec2 TexCoords; // ✅ must match fragment shader

void main() {
    TexCoords = in_pos * 0.5 + 0.5; // [-1, 1] => [0, 1]
    gl_Position = vec4(in_pos, 0.0, 1.0);
}
