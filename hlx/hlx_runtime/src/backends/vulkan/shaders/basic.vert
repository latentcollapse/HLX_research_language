#version 450

// Basic Vertex Shader
// Transforms vertices using MVP matrix
// Passes through color and texture coordinates

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_texcoord;
layout(location = 3) in vec4 in_color;

layout(location = 0) out vec3 frag_position;
layout(location = 1) out vec3 frag_normal;
layout(location = 2) out vec2 frag_texcoord;
layout(location = 3) out vec4 frag_color;

layout(push_constant) uniform PushConstants {
    mat4 model;
    mat4 view;
    mat4 projection;
} constants;

void main() {
    // Transform position
    vec4 world_pos = constants.model * vec4(in_position, 1.0);
    gl_Position = constants.projection * constants.view * world_pos;

    // Pass through to fragment shader
    frag_position = world_pos.xyz;
    frag_normal = mat3(transpose(inverse(constants.model))) * in_normal;
    frag_texcoord = in_texcoord;
    frag_color = in_color;
}
