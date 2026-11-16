struct Uniforms {
    transform: mat4x4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;

@vertex
fn vertex(@location(0) position: vec2<f32>) -> VertexOutput {
    return VertexOutput(uniforms.transform * vec4<f32>(position, 0.0, 1.0), position * 0.5 + 0.5);
}

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(image, image_sampler, uv) * uniforms.color;
}
