#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec3 normal0;

void main() {
    normal0 = normal;
    gl_Position = vec4(position * 0.04, 1.0);
}