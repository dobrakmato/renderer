#version 450

layout(set = 0, binding = 0, input_attachment_index = 0) uniform subpassInput accum_buff;
layout(set = 0, binding = 1, input_attachment_index = 0) uniform subpassInput reveal_buff;

layout(location = 0) out vec4 f_color;

void main() {
    vec4 accum = subpassLoad(accum_buff).rgba;
    float r = subpassLoad(reveal_buff).r;

    f_color = vec4(accum.rgb / min(5e4, max(1e-4, accum.a)), r);
}