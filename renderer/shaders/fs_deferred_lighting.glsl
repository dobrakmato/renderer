#version 450

layout(set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput normal_l_model;
layout(set = 0, binding = 1, input_attachment_index = 1) uniform subpassInput albedo_occlusion;
layout(set = 0, binding = 2, input_attachment_index = 2) uniform subpassInput roughness_metallic;
layout(set = 0, binding = 3, input_attachment_index = 3) uniform subpassInput depth;

layout(location = 0) out vec4 hdr;

layout(push_constant) uniform PushConstants {
    vec4 sun_dir;
} push_constants;

void main() {
    hdr = vec4(subpassLoad(albedo_occlusion).rgb, 1.0);
}







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
