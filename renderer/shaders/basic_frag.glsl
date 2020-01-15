#version 450

layout(location = 0) in vec3 normal;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 f_color;

struct DirectionalLight {
    vec3 direction;
    float intensity;
    vec3 color;
    sampler2D shadowMap;
};

struct PointLight {
    vec3 position;
    vec3 color;
    float intensity;
};


struct SpotLight {
    vec3 position;
    float angle;
    vec3 color;
    float intensity;
};

// material textures
layout(set = 0, binding = 0) uniform sampler2D albedo_map;
// layout(set = 0, binding = 1) uniform sampler2D normal_map;
// layout(set = 0, binding = 2) uniform sampler2D metallic_map;
// layout(set = 0, binding = 3) uniform sampler2D roughness_map;
// layout(set = 0, binding = 4) uniform sampler2D occlusion_map;
// layout(set = 0, binding = 5) uniform sampler2D emission_map;
// layout(set = 0, binding = 6) uniform sampler2D height_map;
// layout(set = 0, binding = 7) uniform MaterialData {
//     vec3 albedo_color;
//     vec3 emissive_color;
//     float alpha_cutoff;
// } material_data;

layout(push_constant) uniform PushConstants {
    float time;
} push_constants;

void main() {
    vec3 dir = normalize(vec3(0, 0.5, 0));
    vec3 color = vec3(0.9, 0.9, 0.6) / 2;
    vec3 result = (dot(normal, dir) * color) + vec3(0.45, 0.45, 0.5);

    vec3 base_color = texture(albedo_map, uv).xyz;

    f_color = vec4(base_color * result, 1.0);
}