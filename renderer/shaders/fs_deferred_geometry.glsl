#version 450

layout(location = 0) in vec3 normal;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 normal_l_model;
layout(location = 1) out vec4 albedo_occlusion;
layout(location = 2) out vec4 roughness_metallic;

// material textures
layout(set = 0, binding = 0) uniform sampler2D albedo_map;
layout(set = 0, binding = 1) uniform sampler2D normal_map;
layout(set = 0, binding = 2) uniform sampler2D displacement_map;
layout(set = 0, binding = 3) uniform sampler2D roughness_map;
layout(set = 0, binding = 4) uniform sampler2D occlusion_map;
layout(set = 0, binding = 5) uniform sampler2D metallic_map;
layout(set = 0, binding = 6) uniform MaterialData {
    vec3 albedo_color;
    float alpha_cutoff;
} material_data;

layout(push_constant) uniform PushConstants {
    vec4 sun_dir;
} push_constants;

void main() {
    vec3 albedo = texture(albedo_map, uv).xyz;
    vec3 normal = texture(normal_map, uv).xyz;
    float roughness = texture(roughness_map, uv).r;
    float metallic = texture(metallic_map, uv).r;
    float occlusion = texture(occlusion_map, uv).r;

    normal_l_model = vec4(normal, 0);
    albedo_occlusion = vec4(albedo, occlusion);
    roughness_metallic = vec4(roughness, metallic, 0, 0);
}