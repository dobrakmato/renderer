#version 450
#include "inc_structs.glsl"
#include "inc_brdf.glsl"

layout(set = 1, binding = 0, input_attachment_index = 0) uniform subpassInput normal_l_model;
layout(set = 1, binding = 1, input_attachment_index = 1) uniform subpassInput albedo_occlusion;
layout(set = 1, binding = 2, input_attachment_index = 2) uniform subpassInput roughness_metallic;
layout(set = 1, binding = 3, input_attachment_index = 3) uniform subpassInput depth;

layout(location = 0) out vec4 hdr;

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
    float roughness = clamp(b3.r, 0.0001, 1.0);// dissalow non-sensical 0 roughness
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
