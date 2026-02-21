#version 450

// PBR Fragment Shader
// Physically Based Rendering with metallic-roughness workflow
// Based on glTF 2.0 PBR specification

layout(location = 0) in vec3 frag_position;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec2 frag_texcoord;
layout(location = 3) in vec4 frag_color;

layout(location = 0) out vec4 out_color;

// Textures
layout(binding = 0) uniform sampler2D tex_base_color;
layout(binding = 1) uniform sampler2D tex_metallic_roughness;
layout(binding = 2) uniform sampler2D tex_normal;
layout(binding = 3) uniform sampler2D tex_occlusion;
layout(binding = 4) uniform sampler2D tex_emissive;

// Lighting and camera
layout(binding = 5) uniform SceneUBO {
    vec3 camera_position;
    vec3 light_direction;
    vec3 light_color;
    float light_intensity;
    vec3 ambient_color;
} scene;

const float PI = 3.14159265359;

// GGX/Trowbridge-Reitz normal distribution function
float distribution_ggx(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float nom = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return nom / max(denom, 0.0000001);
}

// Schlick-GGX geometry function
float geometry_schlick_ggx(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;

    float nom = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return nom / max(denom, 0.0000001);
}

// Smith's method for geometry occlusion
float geometry_smith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = geometry_schlick_ggx(NdotV, roughness);
    float ggx1 = geometry_schlick_ggx(NdotL, roughness);

    return ggx1 * ggx2;
}

// Fresnel-Schlick approximation
vec3 fresnel_schlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

void main() {
    // Sample material properties
    vec4 base_color = texture(tex_base_color, frag_texcoord) * frag_color;
    vec2 metallic_roughness = texture(tex_metallic_roughness, frag_texcoord).bg;
    float metallic = metallic_roughness.x;
    float roughness = metallic_roughness.y;
    float occlusion = texture(tex_occlusion, frag_texcoord).r;
    vec3 emissive = texture(tex_emissive, frag_texcoord).rgb;

    // Normal mapping (simplified - assume tangent space = world space)
    vec3 N = normalize(frag_normal);

    // Calculate vectors
    vec3 V = normalize(scene.camera_position - frag_position);
    vec3 L = normalize(-scene.light_direction);
    vec3 H = normalize(V + L);

    // Calculate F0 (surface reflection at zero incidence)
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, base_color.rgb, metallic);

    // Cook-Torrance BRDF
    float NDF = distribution_ggx(N, H, roughness);
    float G = geometry_smith(N, V, L, roughness);
    vec3 F = fresnel_schlick(max(dot(H, V), 0.0), F0);

    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0);
    vec3 specular = numerator / max(denominator, 0.001);

    // Energy conservation
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - metallic;

    // Outgoing radiance
    float NdotL = max(dot(N, L), 0.0);
    vec3 radiance = scene.light_color * scene.light_intensity;

    vec3 Lo = (kD * base_color.rgb / PI + specular) * radiance * NdotL;

    // Ambient
    vec3 ambient = scene.ambient_color * base_color.rgb * occlusion;

    // Final color
    vec3 color = ambient + Lo + emissive;

    // Tone mapping (simple Reinhard)
    color = color / (color + vec3(1.0));

    // Gamma correction
    color = pow(color, vec3(1.0/2.2));

    out_color = vec4(color, base_color.a);
}
