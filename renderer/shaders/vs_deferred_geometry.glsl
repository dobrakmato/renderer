#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;
layout(location = 3) in vec4 tangent;

layout(location = 0) out vec2 uv0;
layout(location = 1) out mat3 tbn0;

layout(set = 1, binding = 0) uniform MatrixData {
    mat4 model;
    mat4 view;
    mat4 projection;
} matrix_data;

void main() {
    vec3 T = normalize((matrix_data.model * vec4(tangent.xyz, 0.0)).xyz);
    vec3 N = normalize((matrix_data.model * vec4(normal, 0.0)).xyz);
    T = normalize(T - dot(T, N) * N);
    vec3 B = cross(N, T);
    tbn0 = mat3(T, B, N);
    uv0 = uv;
    gl_Position = matrix_data.projection * matrix_data.view * matrix_data.model * vec4(position, 1.0);
}