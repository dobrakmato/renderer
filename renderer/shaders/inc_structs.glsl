const uint MAX_LIGHTS = 100;

struct MaterialData {
    vec3 albedo_color;
    float alpha_cutoff;
    float roughness;
    float metallic;
    float opacity;
    float ior;
};

struct DirectionalLight {
    vec3 direction;
    float intensity;
    vec3 color;
};