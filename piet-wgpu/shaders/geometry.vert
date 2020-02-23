#version 450

layout(set = 0, binding = 0) uniform Transform {
    mat4 transform;
};

struct Primitive {
    vec4 color;
};

layout(set = 0, binding = 1) uniform u_primitives { Primitive primitives[512]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in uint a_prim_id;

layout(location = 0) out vec4 v_color;

void main() {
    gl_Position = transform * vec4(a_position, 0.0, 1.0);
    v_color = primitives[a_prim_id].color;
}
