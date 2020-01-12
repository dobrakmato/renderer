#version 450

layout(location = 0) in vec3 normal;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 f_color;

struct LightData {
    vec3 direction;
    float intensity;
    vec3 color;
    sampler2D shadowMap;
};

struct MaterialData {
    vec3 albedo_color;
    vec3 emission_color;
    sampler2D albedo_map;
    sampler2D normal_map;
    sampler2D metallic_map;
    sampler2D roughness_map;
    sampler2D occlusion_map;
    sampler2D emission_map;
    sampler2D height_map;
};

layout(set = 0, binding = 0) uniform sampler2D base_texture;

layout(push_constant) uniform PushConstants {
    float time;
} push_constants;


void main() {
    vec3 dir = normalize(vec3(0.8, -0.5, -0.8));
    vec3 color = vec3(0.9, 0.9, 0.6) / 2;
    vec3 result = (dot(normal, dir) * color) + vec3(0.45, 0.45, 0.5);

    vec3 base_color = texture(base_texture, uv).xyz;

    f_color = vec4(base_color * result, 1.0);
}