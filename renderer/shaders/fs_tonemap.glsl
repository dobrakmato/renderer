#version 450

layout(set = 1, binding = 0, input_attachment_index = 0) uniform subpassInput hdr_buffer;

layout(location = 0) out vec4 f_color;

vec3 tonemap_hejl(vec3 hdr, float whitePt) {
    vec4 vh = vec4(hdr, whitePt);
    vec4 va = (1.425 * vh) + 0.05f;
    vec4 vf = ((vh * va + 0.004f) / ((vh * (va + 0.55f) + 0.0491f))) - 0.0821f;
    return vf.rgb / vf.www;
}

vec3 ACESFilm(vec3 x) {
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), vec3(0), vec3(1));
}

void main() {
    vec3 hdr = subpassLoad(hdr_buffer).rgb;
    vec3 ldr = ACESFilm(hdr);
    f_color = vec4(ldr, 1.0);
}