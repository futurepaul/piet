#version 450

struct Primitive {
    vec2 start;
    vec2 end;
};

layout(location = 0) in vec2 v_screen_pos;
layout(location = 1) in flat uint v_prim_id;
layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform u_primitives { Primitive primitives[512]; };
layout(set = 1, binding = 1) uniform texture1D t_Color;
layout(set = 1, binding = 2) uniform sampler s_Color;

void main() {
    Primitive prim = primitives[v_prim_id];
    vec2 dir = prim.end - prim.start;
    vec2 scaled_dir = dir / dot(dir, dir);

    float offset = dot(v_screen_pos - prim.start, scaled_dir);
    vec4 tex = texture(sampler1D(t_Color, s_Color), offset);
    out_color = tex;
}
