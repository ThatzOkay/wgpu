use wgpu::util::DeviceExt;

fn main() {
    env_logger::init();
    if std::env::args().any(|arg| arg == "--window") {
        struct App;
        impl winit::application::ApplicationHandler for App {
            fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
                let window = std::sync::Arc::new(
                    event_loop
                        .create_window(
                            winit::window::Window::default_attributes()
                                .with_inner_size(winit::dpi::PhysicalSize::new(64, 64)),
                        )
                        .unwrap(),
                );
                pollster::block_on(run(Some(window), Some(event_loop.owned_display_handle())));
                event_loop.exit();
            }
            fn window_event(
                &mut self,
                _: &winit::event_loop::ActiveEventLoop,
                _: winit::window::WindowId,
                _: winit::event::WindowEvent,
            ) {
            }
        }
        winit::event_loop::EventLoop::new()
            .unwrap()
            .run_app(&mut App)
            .unwrap();
    } else {
        pollster::block_on(run(None, None));
    }
}

async fn run(
    window: Option<std::sync::Arc<winit::window::Window>>,
    display: Option<winit::event_loop::OwnedDisplayHandle>,
) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        display: display.map(|display| Box::new(display) as _),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = window.map(|window| instance.create_surface(window).unwrap());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: surface.as_ref(),
            ..Default::default()
        })
        .await
        .unwrap();
    println!("{:?}", adapter.get_info());
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::downlevel_gles2_defaults(),
            ..Default::default()
        })
        .await
        .unwrap();
    if adapter.get_downlevel_capabilities().shader_model == wgpu::ShaderModel::Sm2 {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _ = device.create_sampler(&wgpu::SamplerDescriptor {
            lod_min_clamp: 1.0,
            ..Default::default()
        });
        assert!(scope.pop().await.is_some());
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let _ = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: None,
            ty: wgpu::QueryType::Occlusion,
            count: 1,
        });
        assert!(scope.pop().await.is_some());
    }
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GLES 2.0 smoke test"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let vertices: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&[u16::MAX, 0, 1, 2, u16::MAX]),
        usage: wgpu::BufferUsages::INDEX,
    });
    let mut uniforms = [0.0f32; 128];
    for base in [0, 64] {
        for diagonal in 0..4 {
            uniforms[base + diagonal * 5] = 1.0;
        }
    }
    uniforms[16..20].copy_from_slice(&[1.0, 0.0, 0.0, 1.0]);
    uniforms[80..84].copy_from_slice(&[0.0, 0.5, 0.0, 1.0]);
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(80),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: Some(wgpu::IndexFormat::Uint16),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth16Unorm,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let sampled = device.create_texture(&descriptor);
    let mut pixels = [0u8; 784];
    for row in 0..4 {
        pixels[row * 256..row * 256 + 16].fill(255);
    }
    queue.write_texture(
        sampled.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(256),
            rows_per_image: Some(4),
        },
        descriptor.size,
    );
    let sampled_view = sampled.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(80),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&sampled_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ..descriptor
    });
    let target_view = target.create_view(&Default::default());
    let surface_texture = surface.as_ref().map(|surface| {
        let config = surface.get_default_config(&adapter, 64, 64).unwrap();
        surface.configure(&device, &config);
        match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            other => panic!("Surface acquisition failed: {other:?}"),
        }
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let copy_source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &[7; 16],
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    let copy_target = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&copy_source, 0, &copy_target, 0, 16);
    encoder.clear_buffer(&copy_target, 4, Some(8));
    for view in core::iter::once(target_view).chain(
        surface_texture
            .as_ref()
            .map(|texture| texture.texture.create_view(&Default::default())),
    ) {
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            size: view.texture().size(),
            format: wgpu::TextureFormat::Depth16Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            ..descriptor
        });
        let depth_view = depth.create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[256]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..5, 0, 0..1);
    }
    let stencil = device.create_texture(&wgpu::TextureDescriptor {
        format: wgpu::TextureFormat::Stencil8,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        ..descriptor
    });
    let stencil_view = stencil.create_view(&Default::default());
    let color_view = target.create_view(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(3),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });
    }
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        descriptor.size,
    );
    queue.submit([encoder.finish()]);
    if let Some(texture) = surface_texture {
        queue.present(texture);
        println!("GLES surface presentation passed");
    }
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.unwrap());
    copy_target
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let result = readback.slice(..).get_mapped_range().unwrap();
    let copied = copy_target.slice(..).get_mapped_range().unwrap();
    assert_eq!(&*copied, &[7, 7, 7, 7, 0, 0, 0, 0, 0, 0, 0, 0, 7, 7, 7, 7]);
    for row in 0..4 {
        for column in 0..4 {
            let pixel = &result[row * 256 + column * 4..][..4];
            assert!(
                pixel[0] == 0 && pixel[1].abs_diff(128) <= 1 && pixel[2] == 0 && pixel[3] == 255,
                "unexpected pixel: {pixel:?}"
            );
        }
    }
    println!("GLES rendering, dynamic uniforms, texture upload and readback passed");
}
