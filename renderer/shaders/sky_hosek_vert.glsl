#version 450

layout(location = 0) in vec3 position;

layout(location = 0) out vec3 position0;

layout(set = 0, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
    vec3 cameraPosition;
} frame_matrix_data;

const float SCALE = 200;

void main() {
    gl_Position = frame_matrix_data.projection * frame_matrix_data.view * vec4(position * SCALE, 1.0);
    gl_Position.z = gl_Position.w - 0.00001;
    position0 = (vec4(position * SCALE, 1.0)).xyz;
}