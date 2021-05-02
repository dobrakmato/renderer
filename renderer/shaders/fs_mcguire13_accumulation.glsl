#version 450
#include "inc_structs.glsl"
#include "inc_brdf.glsl"

layout(location = 0) in vec2 in_uv;
layout(location = 1) in mat3 in_tbn;
layout(location = 4) in vec3 in_wsPosition;
layout(location = 5) in vec3 in_normal;

layout(location = 0) out vec4 accum;
layout(location = 1) out vec4 reveal;

layout(std140, set = 3, binding = 0) uniform Lights {
    DirectionalLight lights[MAX_LIGHTS];
} lights_ubo;

layout(std140, set = 0, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
    vec3 cameraPosition;
} frame_matrix_data;

layout(std140, push_constant) uniform PushConstants {
    vec2 resolution;
    uint light_count;
} push_constants;

// material textures
layout(set = 1, binding = 0) uniform sampler2D albedo_map;
layout(set = 1, binding = 1) uniform sampler2D normal_map;
layout(set = 1, binding = 2) uniform sampler2D displacement_map;
layout(set = 1, binding = 3) uniform sampler2D roughness_map;
layout(set = 1, binding = 4) uniform sampler2D occlusion_map;
layout(set = 1, binding = 5) uniform sampler2D metallic_map;
layout(set = 1, binding = 7) uniform sampler2D opacity_map;

layout(std140, set = 1, binding = 6) uniform TheBlock {
    MaterialData material_data;
};

// unpacks normal from DXT5nm format
vec3 unpack_normal(vec4 packednormal) {
    vec3 normal;
    normal.xy = packednormal.wy * 2 - 1;
    normal.z = sqrt(1.0 - clamp(dot(normal.xy, normal.xy), 0.0, 1.0));
    return normal;
}

float w(float z, float alpha) {
    float n1 = abs(z) / 10;
    float n1_3 = n1 * n1 * n1;

    float n2 = abs(z) / 200;
    float n2_2 = n2 * n2;
    float n2_6 = n2_2 * n2_2 * n2_2;

    float f = 10 / (0.00001 + (n1_3) + (n2_6));
    return alpha * max(0.01, min(3000, f));
}

void main() {
    vec3 albedo = material_data.albedo_color * texture(albedo_map, in_uv).xyz;
    //vec3 normal = texture(normal_map, in_uv).xyz;
    float roughness = material_data.roughness * texture(roughness_map, in_uv).r;
    float metallic = material_data.metallic * texture(metallic_map, in_uv).r;
    float occlusion = texture(occlusion_map, in_uv).r;
    float opacity = material_data.opacity * texture(opacity_map, in_uv).r;
    float displacement = texture(displacement_map, in_uv).r;// todo: remove when vulkano-shaders is fixed
    vec3 position = in_wsPosition;

    /* normal mapping */
    //vec3 n = in_tbn * normalize(normal);

    /* remap roughness */
    roughness = roughness * roughness;

    vec3 N = normalize(in_normal);
    vec3 V = normalize(frame_matrix_data.cameraPosition.xyz - position);

    vec3 lighting = vec3(0.0);
    for (uint i = 0; i < push_constants.light_count; i++) {
        vec3 L = lights_ubo.lights[i].direction;
        vec3 H = normalize(L + V);
        float NdotV = clamp(dot(N, V), 0.0001, 1.0);
        float NdotL = clamp(dot(N, L), 0.0, 1.0);
        float NdotH = clamp(dot(N, H), 0.0, 1.0);
        float LdotH = clamp(dot(L, H), 0.0, 1.0);

        lighting += diffuse(roughness, albedo) + specular(roughness, albedo, metallic, H, NdotV, NdotL, NdotH, LdotH) * lights_ubo.lights[i].color * NdotL;
    }

    vec3 Ci = lighting * opacity;
    float ai = opacity;
    float zi = gl_FragCoord.z;

    accum = vec4(Ci, ai) * w(zi, ai);
    reveal = vec4(ai);
}