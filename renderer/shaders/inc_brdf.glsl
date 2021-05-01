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
    float f90 = clamp(dot(F0, vec3(50.0 * 0.33)), 0.0, 1.0);// todo: replace with material property
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
