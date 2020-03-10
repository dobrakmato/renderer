#version 450

layout(set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput normal_l_model;
layout(set = 0, binding = 1, input_attachment_index = 1) uniform subpassInput albedo_occlusion;
layout(set = 0, binding = 2, input_attachment_index = 2) uniform subpassInput roughness_metallic;
layout(set = 0, binding = 3, input_attachment_index = 3) uniform subpassInput depth;

layout(location = 0) out vec4 hdr;

layout(set = 2, binding = 0) uniform FrameMatrixData {
    mat4 view;
    mat4 projection;
    mat4 invProjection;
    mat4 invView;
} frame_matrix_data;

layout(push_constant) uniform PushConstants {
    vec3 sun_dir;
    vec3 camera_pos;
} push_constants;

vec3 PositionFromDepth(float depth) {
    vec2 coord = gl_FragCoord.xy / vec2(1600, 900);

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
    vec3 normal = b1.rgb * 2 - 1;
    vec3 albedo = b2.rgb;
    float occlusion = b2.a;
    float roughness = b3.r;
    float metallic = b3.g;
    vec3 position = PositionFromDepth(depth);

    vec3 N = normal;
    vec3 L = push_constants.sun_dir.xyz;
    vec3 V = normalize(push_constants.camera_pos.xyz - position);
    vec3 H = normalize(L + V);


    vec3 color = vec3(0.85, 0.85, 0.8);
    vec3 result = max(0.1, dot(push_constants.sun_dir.xyz, normal)) * color * albedo;

    hdr = vec4(result, 1.0);
}
