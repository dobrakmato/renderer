#version 450

layout(location = 0) in vec3 normal;
layout(location = 1) in vec3 position;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform PushConstants {
    vec3 camera_position;
    float time;
} push_constants;

// theory: http://www.cs.utah.edu/~shirley/papers/sunsky/sunsky.pdf

vec3 perez_Yxy(float theta, float gamma, vec3 A, vec3 B, vec3 C, vec3 D, vec3 E) {
    float cos_gamma = cos(gamma);
    return (1.0 + A * exp(B / cos(theta))) * (1.0 + C * exp(D * gamma) + E * cos_gamma * cos_gamma);
}

// t - turbidity
// A, B, C, D, E - distribution coefficients (out)
void distribution_coeffs(float t, out vec3 A, out vec3 B, out vec3 C, out vec3 D, out vec3 E)
{
    A = vec3( 0.1787 * t - 1.4630, -0.0193 * t - 0.2592, -0.0167 * t - 0.2608);
    B = vec3(-0.3554 * t + 0.4275, -0.0665 * t + 0.0008, -0.0950 * t + 0.0092);
    C = vec3(-0.0227 * t + 5.3251, -0.0004 * t + 0.2125, -0.0079 * t + 0.2102);
    D = vec3( 0.1206 * t - 2.5771, -0.0641 * t - 0.8989, -0.0441 * t - 1.6537);
    E = vec3(-0.0670 * t + 0.3703, -0.0033 * t + 0.0452, -0.0109 * t + 0.0529);
}

vec3 zenith_luminance_Xyx(float turbidity, float theta_s) {
    const float pi = 3.14159265359;

    float chi = (4.0 / 9.0 - turbidity / 120.0) * (pi - 2 * theta_s);
    float Y_z = (4.0453 * turbidity - 4.9710) * tan(chi) - 0.2155 * turbidity + 2.4192;

    float T2 = turbidity * turbidity;
    float T = turbidity;
    float theta_s_2 = theta_s * theta_s;
    float theta_s_3 = theta_s_2 * theta_s;

    float x = ( 0.00165 * theta_s_3 - 0.00375 * theta_s_2 + 0.00209 * theta_s + 0.0)     * T2 +
              (-0.02903 * theta_s_3 + 0.06377 * theta_s_2 - 0.03202 * theta_s + 0.00394) * T +
              ( 0.11693 * theta_s_3 - 0.21196 * theta_s_2 + 0.06052 * theta_s + 0.25886);

    float y = ( 0.00275 * theta_s_3 - 0.00610 * theta_s_2 + 0.00317 * theta_s + 0.0)     * T2 +
              (-0.04214 * theta_s_3 + 0.08970 * theta_s_2 - 0.04153 * theta_s + 0.00516) * T +
              ( 0.15346 * theta_s_3 - 0.26756 * theta_s_2 + 0.06670 * theta_s + 0.26688);

    return vec3(Y_z, x, y);
}

// Y_z - luminance at zenith
// sun_dir - direction of light from sun (normalized)
// view_dir - direction of look (normalized)
vec3 sky_luminance_Yxy(float turbidity, vec3 sun_dir, vec3 view_dir) {
    const vec3 up = vec3(0.0, 1.0, 0.0);
    vec3 A, B, C, D, E;
    distribution_coeffs(turbidity, A, B, C, D, E);

    float sun_dot_view = max(0.0, dot(sun_dir, view_dir));
    float sun_dot_up = max(0.0, dot(sun_dir, up));
    float view_dot_up = max(0.0, dot(view_dir, up));

    float gamma = acos(sun_dot_view);
    float theta = acos(view_dot_up);
    float theta_s = acos(sun_dot_up);

    vec3 Y_z = zenith_luminance_Xyx(turbidity, theta_s);

    vec3 perez_theta_gamma = perez_Yxy(theta, gamma, A, B, C, D, E);
    vec3 perez_0_gamma_s = perez_Yxy(0.0, theta_s, A, B, C, D, E);
    return Y_z * perez_theta_gamma / perez_0_gamma_s;
}

vec3 YxyToXYZ(vec3 Yxy) {
    float Y = Yxy.r;
    float x = Yxy.g;
    float y = Yxy.b;

    float X = x * ( Y / y );
    float Z = ( 1.0 - x - y ) * ( Y / y );

    return vec3(X,Y,Z);
}

vec3 XYZToRGB(vec3 XYZ) {
    // CIE/E
    mat3 M = mat3
    (
    2.3706743, -0.9000405, -0.4706338,
    -0.5138850,  1.4253036,  0.0885814,
    0.0052982, -0.0146949,  1.0093968
    );

    return XYZ * M;
}

vec3 YxyToRGB(vec3 Yxy) {
    vec3 XYZ = YxyToXYZ(Yxy);
    vec3 RGB = XYZToRGB(XYZ);
    return RGB;
}


void main() {
    const float turbidity = 2.0;
    float time = push_constants.time * 0.5;
    vec3 sun_dir = normalize(vec3(cos(time), sin(time), 0));
    vec3 view_dir = position - push_constants.camera_position;

    vec3 result = YxyToRGB(sky_luminance_Yxy(turbidity, sun_dir, normalize(view_dir))) * 0.05;
    f_color = vec4(result, 1.0);
}