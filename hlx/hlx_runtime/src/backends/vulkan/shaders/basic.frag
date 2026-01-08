#version 450

// Basic Fragment Shader
// Phong shading with single directional light
// Supports diffuse texture and vertex colors

layout(location = 0) in vec3 frag_position;
layout(location = 1) in vec3 frag_normal;
layout(location = 2) in vec2 frag_texcoord;
layout(location = 3) in vec4 frag_color;

layout(location = 0) out vec4 out_color;

// Uniforms
layout(binding = 0) uniform sampler2D tex_diffuse;

layout(binding = 1) uniform LightingUBO {
    vec3 light_direction;    // Directional light direction
    vec3 light_color;        // Light color
    vec3 ambient_color;      // Ambient light color
    vec3 camera_position;    // Camera position for specular
    float specular_power;    // Shininess
} lighting;

void main() {
    // Normalize interpolated normal
    vec3 N = normalize(frag_normal);

    // Sample diffuse texture
    vec4 tex_color = texture(tex_diffuse, frag_texcoord);

    // Combine texture and vertex color
    vec4 base_color = tex_color * frag_color;

    // Ambient
    vec3 ambient = lighting.ambient_color * base_color.rgb;

    // Diffuse
    vec3 L = normalize(-lighting.light_direction);
    float diff = max(dot(N, L), 0.0);
    vec3 diffuse = diff * lighting.light_color * base_color.rgb;

    // Specular (Phong)
    vec3 V = normalize(lighting.camera_position - frag_position);
    vec3 R = reflect(-L, N);
    float spec = pow(max(dot(V, R), 0.0), lighting.specular_power);
    vec3 specular = spec * lighting.light_color;

    // Combine lighting
    vec3 final_color = ambient + diffuse + specular;

    out_color = vec4(final_color, base_color.a);
}
