#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;

layout(location = 0) out vec3 normal0;
layout(location = 1) out vec2 uv0;

layout(set = 1, binding = 0) uniform MatrixData {
    mat4 model;
    mat4 view;
    mat4 projection;
} matrix_data;

void main() {
    uv0 = uv;
    normal0 = normalize((matrix_data.model * vec4(normal, 0.0)).xyz);
    gl_Position = matrix_data.projection * matrix_data.view * matrix_data.model * vec4(position, 1.0);
}