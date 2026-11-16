#version 100

#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
precision highp int;
#define naga_gles2_precision highp
#else
precision mediump float;
precision mediump int;
#define naga_gles2_precision mediump
#endif

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
uniform Uniforms _group_0_binding_0_fs;

uniform naga_gles2_precision sampler2D _group_0_binding_1_fs;

varying vec2 _vs2fs_location0;

void main() {
    vec2 uv_1 = _vs2fs_location0;
    vec4 color = vec4(0.0);
    vec4 _e3 = texture2D(_group_0_binding_1_fs, vec2(uv_1));
    color = _e3;
    int _e7 = _group_0_binding_0_fs.mode;
    int switch_selector = _e7;
    bool switch_active = false;
    do {
        if (switch_active || (switch_selector == 0)) {
            switch_active = true;
            vec4 _e8 = color;
            vec4 _e12 = _group_0_binding_0_fs.colors[0];
            color = (_e8 * _e12);
            break;
        }
        if (switch_active || (switch_selector == 1)) {
            switch_active = true;
        }
        if (switch_active || (switch_selector == 2)) {
            switch_active = true;
            vec4 _e14 = color;
            vec4 _e18 = _group_0_binding_0_fs.colors[1];
            color = (_e14 * _e18);
            break;
        }
        if (switch_active || (true && switch_selector != 0 && switch_selector != 1 && switch_selector != 2)) {
            switch_active = true;
            color = vec4(naga_gles2_round(uv_1), naga_gles2_trunc(uv_1));
            break;
        }
    } while(false);
    vec4 _e23 = color;
    bvec4 mask = greaterThan(_e23, vec4(0.5));
    vec4 _e27 = color;
    vec4 _e30 = color;
    vec4 selected = vec4(mask.x ? (vec4(1.0) - _e30).x : _e27.x, mask.y ? (vec4(1.0) - _e30).y : _e27.y, mask.z ? (vec4(1.0) - _e30).z : _e27.z, mask.w ? (vec4(1.0) - _e30).w : _e27.w);
    int _e35 = _group_0_binding_0_fs.mode;
    int _e38 = _group_0_binding_0_fs.mode;
    ivec2 signed = ivec2(naga_gles2_clamp(ivec2(_e35, -(_e38)).x, ivec2(-2).x, ivec2(2).x), naga_gles2_clamp(ivec2(_e35, -(_e38)).y, ivec2(-2).y, ivec2(2).y));
    vec2 remainder = (uv_1 - vec2(0.5) * naga_gles2_trunc(uv_1 / vec2(0.5)));
    gl_FragColor = (selected + (vec4(remainder, vec2(signed)) * 0.01));
    return;
}

