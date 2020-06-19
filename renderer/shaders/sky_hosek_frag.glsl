#version 450

layout(location = 0) in vec3 position;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
    vec3 cameraPosition;
} frame_matrix_data;

layout(set = 1, binding = 0) uniform HosekWilkieParams {
    vec3 A;
    vec3 B;
    vec3 C;
    vec3 D;
    vec3 E;
    vec3 F;
    vec3 G;
    vec3 H;
    vec3 I;
    vec3 Z;
    vec3 sun_direction;
} params;

vec3 hosek_wilkie(float cos_theta, float cos_gamma, float gamma) {
    vec3 A = params.A;
    vec3 B = params.B;
    vec3 C = params.C;
    vec3 D = params.D;
    vec3 E = params.E;
    vec3 F = params.F;
    vec3 G = params.G;
    vec3 H = params.H;
    vec3 I = params.I;

    vec3 chi = (1.0 + cos_gamma * cos_gamma) / pow((1 + H * H - 2 * H * cos_gamma), vec3(1.5));

    return (1.0 + A * exp(B / (cos_theta + 0.01))) * (C + D * exp(E * gamma) + F * cos_gamma * cos_gamma + G * chi + I * sqrt(cos_theta));
}

vec3 hosek_wilkie2(vec3 sun_dir, vec3 view_dir) {
    const vec3 up = vec3(0.0, 1.0, 0.0);

    float sun_dot_view = max(0.0, dot(sun_dir, view_dir));
    float view_dot_up = dot(view_dir, up);

    float gamma = acos(sun_dot_view);

    return hosek_wilkie(view_dot_up, sun_dot_view, gamma) * params.Z;
}

void main() {
    vec3 view_dir = position - frame_matrix_data.cameraPosition;

    vec3 result = hosek_wilkie2(params.sun_direction, normalize(view_dir)) * 0.05;
    f_color = vec4(result, 1.0);
}