#version 450

layout(location = 0) in vec3 position;

layout(location = 0) out vec3 position0;

layout(set = 0, binding = 0) uniform MatrixData {
    mat4 model;
    mat4 view;
    mat4 projection;
} matrix_data;

void main() {
    gl_Position = matrix_data.projection * matrix_data.view * matrix_data.model * vec4(position, 1.0);
    position0 = (matrix_data.model * vec4(position, 1.0)).xyz;
}