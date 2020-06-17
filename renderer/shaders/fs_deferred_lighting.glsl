#version 450

layout(set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput normal_l_model;
layout(set = 0, binding = 1, input_attachment_index = 1) uniform subpassInput albedo_occlusion;
layout(set = 0, binding = 2, input_attachment_index = 2) uniform subpassInput roughness_metallic;
layout(set = 0, binding = 3, input_attachment_index = 3) uniform subpassInput depth;

layout(location = 0) out vec4 hdr;

const uint MAX_LIGHTS = 1024;

struct DirectionalLight {
    vec3 direction;
    float intensity;
    vec3 color;
};

layout(set = 1, binding = 0) uniform Lights {
    DirectionalLight lights[MAX_LIGHTS];
} lights_ubo;

layout(set = 2, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
} frame_matrix_data;

layout(push_constant) uniform PushConstants {
    vec3 camera_pos;
    vec2 resolution;
    uint light_count;
} push_constants;


vec3 PositionFromDepth(float depth) {
    vec2 coord = gl_FragCoord.xy / push_constants.resolution;

    vec4 clipSpacePosition = vec4(coord * 2.0 - 1.0, depth, 1.0);
    vec4 viewSpacePosition = frame_matrix_data.invProjection * clipSpacePosition;
    viewSpacePosition /= viewSpacePosition.w;
    vec4 worldSpacePosition = frame_matrix_data.invView * viewSpacePosition;
    return worldSpacePosition.xyz;
}


vec3 light(vec3 N, vec3 L, vec3 V, vec3 color, float roughness, vec3 albedo, float metallic) {
    vec3 H = normalize(L + V);
    float alpha = roughness * roughness;

    float NdotV = clamp(dot(N, V), 0.0, 1.0);
    float NdotL = clamp(dot(N, L), 0.0, 1.0);
    float NdotH = clamp(dot(N, H), 0.0, 1.0);
    float VdotH = clamp(dot(V, H), 0.0, 1.0);

    float alphaSq = alpha * alpha;
    float f = (NdotH * alphaSq - NdotH) * NdotH + 1.0;
    float D = alphaSq / (3.14159 * f * f);

    const vec3 dielectricSpecular = vec3(0.04, 0.04, 0.04);
    const vec3 black = vec3(0.0, 0.0, 0.0);

    vec3 F0 = mix(dielectricSpecular, albedo, metallic);
    vec3 F = (F0 + (1 - F0) * pow(clamp(1.0 - VdotH, 0.0, 1.0), 5.0));

    // float GGXV = NdotL * sqrt(NdotV * NdotV * (1.0 - alphaSq) + alphaSq);
    // float GGXL = NdotV * sqrt(NdotL * NdotL * (1.0 - alphaSq) + alphaSq);
    // float GGX = GGXL + GGXV;
    // float G = 0.5 / (GGX);

    float k = (alpha + 2 * roughness + 1) / 8.0;
    float G = NdotL / (mix(NdotL, 1, k) * mix(NdotV, 1, k));

    // float Vis = G / (4 * NdotL * NdotV);
    float Vis = G / 4.0;

    vec3 diffuse = mix(albedo * (vec3(1.0, 1.0, 1.0) - F0), black, metallic) / 3.14159;

    vec3 specularContribution = D * Vis * F;
    vec3 diffuseContribution = (1.0 - F) * diffuse;

    return (specularContribution + diffuseContribution) * color * NdotL;

}

void main() {
    /* load data from buffers */
    vec4 b1 = subpassLoad(normal_l_model);
    vec4 b2 = subpassLoad(albedo_occlusion);
    vec4 b3 = subpassLoad(roughness_metallic);
    float depth = subpassLoad(depth).x;

    /* unpack the individual components */
    vec3 normal = b1.rgb * 2 - 1.0;
    vec3 albedo = b2.rgb;
    float occlusion = b2.a;
    float roughness = clamp(b3.r, 0.0, 1.0);
    float metallic = b3.g;
    vec3 position = PositionFromDepth(depth);

    vec3 N = normalize(normal);
    vec3 V = normalize(push_constants.camera_pos.xyz - position);

    vec3 result = vec3(0.0);
    for (uint i = 0; i < push_constants.light_count; i++) {
        result += light(N, lights_ubo.lights[i].direction, V, lights_ubo.lights[i].color, roughness, albedo, metallic) * lights_ubo.lights[i].intensity * occlusion;
    }

    hdr = vec4(result, 1.0);
}
