#version 450

layout(location = 0) in vec3 normal;

layout(location = 0) out vec4 f_color;

void main() {
    vec3 dir = normalize(vec3(0.45, -0.8, 0.6));
    vec3 color = vec3(0.9, 0.9, 0.88);

    f_color = vec4(dot(normal, dir) * color + vec3(0.05, 0.05, 0.1), 1.0);
}