#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;
layout(location = 3) in vec4 tangent;

layout(location = 0) out vec2 uv0;
layout(location = 1) out mat3 tbn0;
layout(location = 4) out vec3 wsPosition0;

layout(std140, set = 0, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
    vec3 cameraPosition;
} frame_matrix_data;

layout(std140, set = 2, binding = 0) uniform ObjectMatrixData {
    mat4 model;
} object_matrix_data;

void main() {
    vec3 T = normalize((object_matrix_data.model * vec4(tangent.xyz, 0.0)).xyz);
    vec3 N = normalize((object_matrix_data.model * vec4(normal, 0.0)).xyz);
    T = normalize(T - dot(T, N) * N);
    vec3 B = cross(N, T);
    tbn0 = mat3(T, B, N);
    vec4 wsPosition = object_matrix_data.model * vec4(position, 1.0);
    wsPosition0 = wsPosition.xyz;
    uv0 = uv;
    gl_Position = frame_matrix_data.projection * frame_matrix_data.view * wsPosition;
}