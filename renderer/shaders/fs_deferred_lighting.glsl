#version 450

layout(set = 1, binding = 0, input_attachment_index = 0) uniform subpassInput normal_l_model;
layout(set = 1, binding = 1, input_attachment_index = 1) uniform subpassInput albedo_occlusion;
layout(set = 1, binding = 2, input_attachment_index = 2) uniform subpassInput roughness_metallic;
layout(set = 1, binding = 3, input_attachment_index = 3) uniform subpassInput depth;

layout(location = 0) out vec4 hdr;

const uint MAX_LIGHTS = 100;

struct DirectionalLight {
    vec3 direction;
    float intensity;
    vec3 color;
};

layout(std140, set = 2, binding = 0) uniform Lights {
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


// extract position from depth value
vec3 PositionFromDepth(float depth) {
    vec2 coord = gl_FragCoord.xy / push_constants.resolution;

    vec4 clipSpacePosition = vec4(coord * 2.0 - 1.0, depth, 1.0);
    vec4 viewSpacePosition = frame_matrix_data.invProjection * clipSpacePosition;
    viewSpacePosition /= viewSpacePosition.w;
    vec4 worldSpacePosition = frame_matrix_data.invView * viewSpacePosition;
    return worldSpacePosition.xyz;
}

// ggx distribution term
float D_GGX(float roughness, float NdotH, const vec3 H) {
    float oneMinusNoHSquared = 1.0 - NdotH * NdotH;
    float a = NdotH * roughness;
    float k = roughness / (oneMinusNoHSquared + a * a);
    float d = k * k * (1.0 / 3.14159);
    return d;
}

float distribution(float roughness, float NdotH, const vec3 H) {
    return D_GGX(roughness, NdotH, H);
}

float V_SmithGGXCorrelated(float roughness, float NdotV, float NdotL) {
    float a2 = roughness * roughness;
    float GGXV = NdotL * sqrt((NdotV - a2 * NdotV) * NdotV + a2);
    float GGXL = NdotV * sqrt((NdotL - a2 * NdotL) * NdotL + a2);
    return 0.5 / (GGXV + GGXL);
}

float visibility(float roughness, float NoV, float NoL) {
    return V_SmithGGXCorrelated(roughness, NoV, NoL);
}

vec3 F_Schlick(const vec3 F0, float F90, float VdotH) {
    return F0 + (F90 - F0) * pow(1.0 - VdotH, 5);
}

vec3 fresnel(const vec3 F0, float LdotH) {
    float f90 = clamp(dot(F0, vec3(50.0 * 0.33)), 0.0, 1.0); // todo: replace with material property
    return F_Schlick(F0, f90, LdotH);
}

vec3 specular(float roughness, vec3 albedo, float metallic, const vec3 h, float NdotV, float NdotL, float NdotH, float LdotH) {
    const vec3 dielectricSpecular = vec3(0.04, 0.04, 0.04);
    vec3 F0 = mix(dielectricSpecular, albedo, metallic);

    float D = distribution(roughness, NdotH, h);
    float V = visibility(roughness, NdotV, NdotL);
    vec3  F = fresnel(F0, LdotH);

    return (D * V) * F;
}

vec3 diffuse(float roughness, vec3 albedo) {
    return albedo / 3.14159;
}

vec3 light(vec3 N, vec3 L, vec3 V, vec3 lightColor, float roughness, vec3 albedo, float metallic) {
    vec3 H = normalize(L + V);

    float NdotV = clamp(dot(N, V), 0.0001, 1.0);
    float NdotL = clamp(dot(N, L), 0.0, 1.0);
    float NdotH = clamp(dot(N, H), 0.0, 1.0);
    float LdotH = clamp(dot(L, H), 0.0, 1.0);

    vec3 specular = specular(roughness, albedo, metallic, H, NdotV, NdotL, NdotH, LdotH);
    vec3 diffuse = diffuse(roughness, albedo);

    vec3 color = diffuse * (1 - metallic) + mix(specular, specular * albedo, metallic);

    return (color * lightColor) * NdotL;
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
    float roughness = clamp(b3.r, 0.00001, 1.0); // dissalow non-sensical 0 roughness
    float metallic = b3.g;
    vec3 position = PositionFromDepth(depth);

    /* remap roughness */
    roughness = roughness * roughness;

    vec3 N = normalize(normal);
    vec3 V = normalize(frame_matrix_data.cameraPosition.xyz - position);

    vec3 result = vec3(0.0);
    for (uint i = 0; i < push_constants.light_count; i++) {
        result += (light(N, lights_ubo.lights[i].direction, V, lights_ubo.lights[i].color, roughness, albedo, metallic) * lights_ubo.lights[i].intensity * occlusion);
    }

    hdr = vec4(result, 1.0);
}
