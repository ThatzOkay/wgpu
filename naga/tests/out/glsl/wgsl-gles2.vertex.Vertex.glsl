#version 100

precision highp float;
precision highp int;

int naga_gles2_abs(int x) { return x < 0 ? -x : x; }
int naga_gles2_sign(int x) { return x < 0 ? -1 : (x > 0 ? 1 : 0); }
int naga_gles2_min(int x, int y) { return x < y ? x : y; }
int naga_gles2_max(int x, int y) { return x > y ? x : y; }
int naga_gles2_clamp(int x, int lo, int hi) { return naga_gles2_min(naga_gles2_max(x, lo), hi); }
float naga_gles2_trunc(float x) { return sign(x) * floor(abs(x)); }
float naga_gles2_round(float x) { return mix(floor(x + float(0.5)), float(2.0) * floor(x * float(0.5) + float(0.5)), step(abs(fract(x) - float(0.5)), float(0.0))); }
vec2 naga_gles2_trunc(vec2 x) { return sign(x) * floor(abs(x)); }
vec2 naga_gles2_round(vec2 x) { return mix(floor(x + vec2(0.5)), vec2(2.0) * floor(x * vec2(0.5) + vec2(0.5)), step(abs(fract(x) - vec2(0.5)), vec2(0.0))); }
vec3 naga_gles2_trunc(vec3 x) { return sign(x) * floor(abs(x)); }
vec3 naga_gles2_round(vec3 x) { return mix(floor(x + vec3(0.5)), vec3(2.0) * floor(x * vec3(0.5) + vec3(0.5)), step(abs(fract(x) - vec3(0.5)), vec3(0.0))); }
vec4 naga_gles2_trunc(vec4 x) { return sign(x) * floor(abs(x)); }
vec4 naga_gles2_round(vec4 x) { return mix(floor(x + vec4(0.5)), vec4(2.0) * floor(x * vec4(0.5) + vec4(0.5)), step(abs(fract(x) - vec4(0.5)), vec4(0.0))); }
struct Uniforms {
    mat4 transform;
    vec4 colors[2];
    int mode;
};
struct VertexOutput {
    vec4 position;
    vec2 uv;
};
uniform Uniforms _group_0_binding_0_vs;

attribute vec2 _p2vs_location0;
attribute vec2 _p2vs_location1;
varying vec2 _vs2fs_location0;

void main() {
    vec2 position = _p2vs_location0;
    vec2 uv = _p2vs_location1;
    mat4 _e4 = _group_0_binding_0_vs.transform;
    VertexOutput _tmp_return = VertexOutput((_e4 * vec4(position, 0.0, 1.0)), uv);
    gl_Position = _tmp_return.position;
    _vs2fs_location0 = _tmp_return.uv;
    gl_Position.yz = vec2(-gl_Position.y, gl_Position.z * 2.0 - gl_Position.w);
    gl_PointSize = 1.0;
    return;
}

