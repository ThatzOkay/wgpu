# GLES 2.0 rendering

This example renders a textured triangle, checks its pixels, and optionally
presents it in a window. It exercises vertex and index buffer uploads, dynamic
uniform offsets, depth attachments, strip restart indices, buffer copies and
clears, texture uploads with padded rows, and texture readback.

Run on the device with:

```sh
cargo run -p wgpu-example-04-gles2
cargo run -p wgpu-example-04-gles2 -- --window
```

On Mesa, force GLES 2.0 instead of a newer context with:

```sh
MESA_GL_VERSION_OVERRIDE=2.1 MESA_GLES_VERSION_OVERRIDE=2.0 \
    cargo run -p wgpu-example-04-gles2
```

The desktop GL override prevents EGL from selecting the normal desktop GL 3.3
path. It does not select a desktop OpenGL 2.1 backend. On a headless Mesa system,
also set `EGL_PLATFORM=surfaceless`.

For an application, enable the `gles` and `wgsl` crate features, select
`Backends::GL`, and request `Limits::downlevel_gles2_defaults()`. Supply the
window system's display handle when creating the instance for presentation.
With automatic context selection, EGL tries GLES 2.0 if newer contexts fail.
Explicitly requested GLES 3.x versions do not fall back.

The GLES 2.0 profile provides:

- Vertex and fragment shaders using GLSL ES 1.00-compatible WGSL features.
- Floating-point and normalized vertex attributes, with 16-bit indices.
- A single `Rgba8Unorm` color attachment, `Depth16Unorm` depth attachments, and
  `Stencil8` stencil attachments.
- Sampled RGBA8 2D and cube textures, with automatic sampling and bias.
- Uniform buffers, including dynamic offsets, arrays, and nested structures.
- Ordinary uniform uploads for immediate data.
- Buffer copies, clears, texture uploads and readback through CPU memory.
- EGL window presentation through a fullscreen textured triangle.

This profile does not provide WebGPU conformance. Applications must account for
the GLES 2.0 restrictions:

- No compute, storage resources, queries, multisampling, texture arrays, integer
  textures, comparison samplers, min/max blending, or instanced vertex buffers.
- Shaders cannot use unsigned integers, integer bit operations, bitcasts,
  non-square matrices, array values/constructors, or shader-generated vertex and
  instance indices. Use vertex buffers for vertex positions.
- GLSL ES 3.x-only mathematical builtins are rejected unless the backend has a
  fallback. Derivatives, fragment depth writes, and explicit fragment texture
  LOD require extensions and are not exposed by this core profile.
- Sampled texture views must include the texture's complete mip range. Render
  attachment views may select individual mip levels or cube faces.
- Non-power-of-two textures require clamp-to-edge addressing and no mipmaps.
  Sampler LOD clamps must retain the default range, with a zero minimum and a
  maximum of at least 32. Filter modes and repeat addressing for power-of-two
  textures use ordinary GLES 2.0 texture parameters.
- A texture object cannot be sampled simultaneously with different sampler
  states. Use separate texture objects when independent states are needed.
- Fragment high precision is optional; generated shaders use medium precision
  when `GL_FRAGMENT_PRECISION_HIGH` is unavailable. Integer precision and loop
  limits remain subject to the device's GLSL ES 1.00 implementation.
- Buffer contents are mirrored in CPU memory, and submissions use `glFinish`
  because core GLES 2.0 has no GL sync objects.

The smoke test has been verified using Mesa contexts restricted to GLES 2.0,
including EGL window presentation, and GLES 3.0. Tegra 3 driver behavior has not
been verified on hardware.

The implementation follows the [OpenGL ES 2.0 specification](https://registry.khronos.org/OpenGL/specs/es/2.0/es_full_spec_2.0.pdf),
the [GLSL ES 1.00 specification](https://www.khronos.org/files/opengles_shading_language.pdf),
and [WGSL memory layout](https://www.w3.org/TR/WGSL/#memory-layout).
