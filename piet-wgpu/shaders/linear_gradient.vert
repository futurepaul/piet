#version 450

layout(set = 0, binding = 0) uniform Transform {
    mat4 transform;
};

struct Primitive {
    vec2 start;
    vec2 end;
};

layout(set = 1, binding = 0) uniform u_primitives { Primitive primitives[512]; };

layout(location = 0) in vec2 a_position;
layout(location = 1) in uint a_prim_id;

layout(location = 0) out vec2 v_screen_pos;
layout(location = 1) out flat uint v_prim_id;

void main() {
    gl_Position = transform * vec4(a_position, 0.0, 1.0);
    v_screen_pos = a_position;
    v_prim_id = a_prim_id;
}
