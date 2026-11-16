struct Uniforms {
    transform: mat4x4<f32>,
    colors: array<vec4<f32>, 2>,
    mode: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;

@vertex
fn vertex(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    return VertexOutput(uniforms.transform * vec4<f32>(position, 0.0, 1.0), uv);
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    var color = textureSample(image, image_sampler, uv);
    switch uniforms.mode {
        case 0: { color *= uniforms.colors[0]; }
        case 1, 2: { color *= uniforms.colors[1]; }
        default: { color = vec4<f32>(round(uv), trunc(uv)); }
    }
    let mask = color > vec4<f32>(0.5);
    let selected = select(color, vec4<f32>(1.0) - color, mask);
    let signed = clamp(vec2<i32>(uniforms.mode, -uniforms.mode), vec2<i32>(-2), vec2<i32>(2));
    let remainder = uv % vec2<f32>(0.5);
    return selected + vec4<f32>(remainder, vec2<f32>(signed)) * 0.01;
}
