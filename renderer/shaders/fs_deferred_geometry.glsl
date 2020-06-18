#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 1) in mat3 in_tbn;

layout(location = 0) out vec4 normal_l_model;
layout(location = 1) out vec4 albedo_occlusion;
layout(location = 2) out vec4 roughness_metallic;

// material textures
layout(set = 1, binding = 0) uniform sampler2D albedo_map;
layout(set = 1, binding = 1) uniform sampler2D normal_map;
layout(set = 1, binding = 2) uniform sampler2D displacement_map;
layout(set = 1, binding = 3) uniform sampler2D roughness_map;
layout(set = 1, binding = 4) uniform sampler2D occlusion_map;
layout(set = 1, binding = 5) uniform sampler2D metallic_map;
layout(set = 1, binding = 6) uniform MaterialData {
    vec3 albedo_color;
    float alpha_cutoff;
    float roughness;
    float metallic;
} material_data;

// unpacks normal from DXT5nm format
vec3 unpack_normal(vec4 packednormal) {
    vec3 normal;
    normal.xy = packednormal.wy * 2 - 1;
    normal.z = sqrt(1.0 - clamp(dot(normal.xy, normal.xy), 0.0, 1.0));
    return normal;
}

void main() {
    vec3 albedo = material_data.albedo_color * texture(albedo_map, in_uv).xyz;
    vec3 normal = unpack_normal(texture(normal_map, in_uv));
    float roughness = material_data.roughness * texture(roughness_map, in_uv).r;
    float metallic = material_data.metallic * texture(metallic_map, in_uv).r;
    float occlusion = texture(occlusion_map, in_uv).r;

    vec3 n = in_tbn * normalize(normal);

    normal_l_model = vec4(n * 0.5 + 0.5, 0);
    albedo_occlusion = vec4(albedo, occlusion);
    roughness_metallic = vec4(roughness, metallic, 0, 0);
}