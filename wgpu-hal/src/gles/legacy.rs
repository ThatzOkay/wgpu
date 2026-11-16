use super::*;
use alloc::collections::BTreeMap;
use glow::HasContext;

#[derive(Debug, Default)]
pub(super) struct State {
    pub uniforms: Vec<(u8, ImmediateDesc)>,
    pub buffers: BTreeMap<u8, (Buffer, i32, i32)>,
    pub textures: BTreeMap<u32, (glow::Texture, u32, u32)>,
    pub samplers: BTreeMap<u32, SamplerParameters>,
    pub index_buffer: Option<Buffer>,
}

#[derive(Clone, Debug)]
pub(super) struct SamplerParameters {
    min: u32,
    mag: u32,
    wrap: [u32; 2],
}

impl SamplerParameters {
    pub fn new(desc: &crate::SamplerDescriptor) -> Self {
        let (min, mag) =
            conv::map_filter_modes(desc.min_filter, desc.mag_filter, desc.mipmap_filter);
        Self {
            min,
            mag,
            wrap: [
                conv::map_address_mode(desc.address_modes[0]),
                conv::map_address_mode(desc.address_modes[1]),
            ],
        }
    }
}

impl AdapterShared {
    pub(super) fn is_gles2(&self) -> bool {
        self.shading_language_version.is_gles2()
    }
}

impl Queue {
    pub(super) unsafe fn draw_gles2_indexed(
        &self,
        gl: &glow::Context,
        topology: u32,
        count: u32,
        offset: u32,
        instances: u32,
    ) {
        if matches!(topology, glow::TRIANGLE_STRIP | glow::LINE_STRIP) {
            let state = self.legacy_state.lock();
            let buffer = state.index_buffer.as_ref().unwrap();
            let data = buffer.map_state.lock();
            let data = &data.data.as_ref().unwrap()[offset as usize..][..count as usize * 2];
            for _ in 0..instances {
                let mut start = 0;
                for index in 0..=count {
                    if index == count
                        || data[index as usize * 2..index as usize * 2 + 2] == [255, 255]
                    {
                        if index > start {
                            unsafe {
                                gl.draw_elements(
                                    topology,
                                    (index - start) as i32,
                                    glow::UNSIGNED_SHORT,
                                    (offset + start * 2) as i32,
                                )
                            };
                        }
                        start = index + 1;
                    }
                }
            }
        } else {
            for _ in 0..instances {
                unsafe {
                    gl.draw_elements(topology, count as i32, glow::UNSIGNED_SHORT, offset as i32)
                };
            }
        }
    }

    pub(super) fn draw_framebuffer(&self) -> u32 {
        if self.shared.is_gles2() {
            glow::FRAMEBUFFER
        } else {
            glow::DRAW_FRAMEBUFFER
        }
    }

    pub(super) fn read_framebuffer(&self) -> u32 {
        if self.shared.is_gles2() {
            glow::FRAMEBUFFER
        } else {
            glow::READ_FRAMEBUFFER
        }
    }

    pub(super) unsafe fn copy_gles2_texture(
        &self,
        gl: &glow::Context,
        buffer: &Buffer,
        texture: glow::Texture,
        target: u32,
        copy: &crate::BufferTextureCopy,
        upload: bool,
    ) {
        let row_size = copy.size.width as usize * 4;
        let row_stride = copy
            .buffer_layout
            .bytes_per_row
            .map_or(row_size, |value| value as usize);
        let image_stride = row_stride
            * copy
                .buffer_layout
                .rows_per_image
                .unwrap_or(copy.size.height) as usize;
        let mut map_state = buffer.map_state.lock();
        let data = map_state.data.as_mut().unwrap();
        unsafe { gl.bind_texture(target, Some(texture)) };
        if !upload {
            unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.copy_fbo)) };
        }
        for layer in 0..copy.size.depth {
            let face = if target == glow::TEXTURE_CUBE_MAP {
                glow::TEXTURE_CUBE_MAP_POSITIVE_X + copy.texture_base.array_layer + layer
            } else {
                target
            };
            if !upload {
                unsafe {
                    gl.framebuffer_texture_2d(
                        glow::FRAMEBUFFER,
                        glow::COLOR_ATTACHMENT0,
                        face,
                        Some(texture),
                        copy.texture_base.mip_level as i32,
                    )
                };
            }
            for row in 0..copy.size.height {
                let offset = copy.buffer_layout.offset as usize
                    + layer as usize * image_stride
                    + row as usize * row_stride;
                let bytes = &mut data[offset..offset + row_size];
                unsafe {
                    if upload {
                        gl.tex_sub_image_2d(
                            face,
                            copy.texture_base.mip_level as i32,
                            copy.texture_base.origin.x as i32,
                            (copy.texture_base.origin.y + row) as i32,
                            copy.size.width as i32,
                            1,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
                            glow::PixelUnpackData::Slice(Some(bytes)),
                        );
                    } else {
                        gl.read_pixels(
                            copy.texture_base.origin.x as i32,
                            (copy.texture_base.origin.y + row) as i32,
                            copy.size.width as i32,
                            1,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
                            glow::PixelPackData::Slice(Some(bytes)),
                        );
                    }
                }
            }
        }
        if !upload {
            unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.draw_fbo)) };
            if let Some(raw) = buffer.raw {
                unsafe {
                    gl.bind_buffer(buffer.target, Some(raw));
                    gl.buffer_sub_data_u8_slice(buffer.target, 0, data);
                }
            }
        }
    }

    pub(super) unsafe fn prepare_gles2_draw(&self, gl: &glow::Context) {
        let state = self.legacy_state.lock();
        for (slot, uniform) in &state.uniforms {
            let Some((buffer, base, size)) = state.buffers.get(slot) else {
                continue;
            };
            assert!(uniform.offset + uniform.size_bytes <= *size as u32);
            let data = buffer.map_state.lock();
            let command = Command::SetImmediates {
                uniform: uniform.clone(),
                offset: *base as u32 + uniform.offset,
            };
            unsafe { self.process(gl, &command, data.data.as_ref().unwrap(), &[]) };
        }
        for (slot, &(texture, target, levels)) in &state.textures {
            let Some(sampler) = state.samplers.get(slot) else {
                continue;
            };
            let min = if levels == 1 {
                match sampler.min {
                    glow::NEAREST_MIPMAP_NEAREST | glow::NEAREST_MIPMAP_LINEAR => glow::NEAREST,
                    glow::LINEAR_MIPMAP_NEAREST | glow::LINEAR_MIPMAP_LINEAR => glow::LINEAR,
                    other => other,
                }
            } else {
                sampler.min
            };
            unsafe {
                gl.active_texture(glow::TEXTURE0 + *slot);
                gl.bind_texture(target, Some(texture));
                gl.tex_parameter_i32(target, glow::TEXTURE_MIN_FILTER, min as i32);
                gl.tex_parameter_i32(target, glow::TEXTURE_MAG_FILTER, sampler.mag as i32);
                gl.tex_parameter_i32(target, glow::TEXTURE_WRAP_S, sampler.wrap[0] as i32);
                gl.tex_parameter_i32(target, glow::TEXTURE_WRAP_T, sampler.wrap[1] as i32);
            }
        }
    }

    pub(super) unsafe fn write_gles2_buffer(
        &self,
        gl: &glow::Context,
        buffer: &Buffer,
        offset: usize,
        data: &[u8],
    ) {
        buffer.map_state.lock().data.as_mut().unwrap()[offset..offset + data.len()]
            .copy_from_slice(data);
        if let Some(raw) = buffer.raw {
            unsafe {
                gl.bind_buffer(buffer.target, Some(raw));
                gl.buffer_sub_data_u8_slice(buffer.target, offset as i32, data);
                if buffer.target == glow::ELEMENT_ARRAY_BUFFER {
                    gl.bind_buffer(buffer.target, *self.current_index_buffer.lock());
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct Presenter {
    program: glow::Program,
    vertices: glow::Buffer,
}

impl Presenter {
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, crate::DeviceError> {
        let program =
            unsafe { gl.create_program() }.map_err(|_| crate::DeviceError::OutOfMemory)?;
        let sources = [
            (glow::VERTEX_SHADER, "#version 100\nattribute vec2 position; varying vec2 uv; void main() { gl_Position = vec4(position, 0.0, 1.0); uv = vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5); }"),
            (glow::FRAGMENT_SHADER, "#version 100\nprecision mediump float; varying vec2 uv; uniform sampler2D image; void main() { gl_FragColor = texture2D(image, uv); }"),
        ];
        for (stage, source) in sources {
            let shader =
                unsafe { gl.create_shader(stage) }.map_err(|_| crate::DeviceError::OutOfMemory)?;
            unsafe {
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
            }
            if !unsafe { gl.get_shader_compile_status(shader) } {
                log::error!("GLES 2.0 presentation shader: {}", unsafe {
                    gl.get_shader_info_log(shader)
                });
                unsafe {
                    gl.delete_shader(shader);
                    gl.delete_program(program);
                }
                return Err(crate::DeviceError::Lost);
            }
            unsafe {
                gl.attach_shader(program, shader);
                gl.delete_shader(shader);
            }
        }
        unsafe {
            gl.bind_attrib_location(program, 0, "position");
            gl.link_program(program);
        }
        if !unsafe { gl.get_program_link_status(program) } {
            log::error!("GLES 2.0 presentation program: {}", unsafe {
                gl.get_program_info_log(program)
            });
            unsafe { gl.delete_program(program) };
            return Err(crate::DeviceError::Lost);
        }
        let vertices =
            unsafe { gl.create_buffer() }.map_err(|_| crate::DeviceError::OutOfMemory)?;
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&[-1.0f32, -1.0, 3.0, -1.0, -1.0, 3.0]),
                glow::STATIC_DRAW,
            );
            gl.use_program(Some(program));
            let location = gl.get_uniform_location(program, "image");
            gl.uniform_1_i32(location.as_ref(), 0);
        }
        Ok(Self { program, vertices })
    }

    pub unsafe fn draw(&self, gl: &glow::Context, texture: glow::Texture, extent: wgt::Extent3d) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, extent.width as i32, extent.height as i32);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::STENCIL_TEST);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.disable_vertex_attrib_array(0);
        }
    }

    pub unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_buffer(self.vertices);
        }
    }
}
