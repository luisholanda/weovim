#version 450

layout(location = 0) in vec2 v_Pos;
layout(location = 1) in float i_Width;
layout(location = 2) in float i_Height;
layout(location = 3) in vec4 i_Color;

const vec2 vertices[4] = vec2[4](
    vec2(-0.5, -0.5),
    vec2(-0.5,  0.5),
    vec2( 0.5,  0.5),
    vec2( 0.5, -0.5)
);

void main() {
    mat4 p_Transform = mat4(
        vec4(i_Width,      0.0, 0.0, 0.0),
        vec4(    0.0, i_Height, 0.0, 0.0),
        vec4(    0.0,      0.0, 1.0, 0.0),
        vec4(       v_Pos     , 0.0, 1.0)
    );

    gl_Position = p_Transform * vec4(vertices[gl_VertexIndex], 0.0, 1.0);
}
