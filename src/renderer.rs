use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use log::{debug, info};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::terminal::Terminal;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x4
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct GlyphInfo {
    texture_coords: [f32; 4],
    size: [f32; 2],
    bearing: [f32; 2],
    advance: f32,
}

pub struct TextureAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    current_x: u32,
    current_y: u32,
    row_height: u32,
}

impl TextureAtlas {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("glyph_atlas"),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            width,
            height,
            current_x: 0,
            current_y: 0,
            row_height: 0,
        }
    }

    fn add_glyph(&mut self, queue: &wgpu::Queue, glyph_data: &[u8], width: u32, height: u32) -> Option<[f32; 4]> {
        if self.current_x + width > self.width {
            self.current_x = 0;
            self.current_y += self.row_height;
            self.row_height = 0;
        }

        if self.current_y + height > self.height {
            return None;
        }

        let tex_coords = [
            self.current_x as f32 / self.width as f32,
            self.current_y as f32 / self.height as f32,
            (self.current_x + width) as f32 / self.width as f32,
            (self.current_y + height) as f32 / self.height as f32,
        ];

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: self.current_x,
                    y: self.current_y,
                    z: 0,
                },
            },
            glyph_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.current_x += width;
        self.row_height = self.row_height.max(height);

        Some(tex_coords)
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    font: Font,
    glyph_cache: HashMap<char, GlyphInfo>,
    texture_atlas: TextureAtlas,
    font_size: f32,
    scale_factor: f64,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        
        info!("Initializing renderer with size: {:?}, scale factor: {}", size, scale_factor);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window)?) }?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable adapter"))?;

        debug!("Adapter info: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let font_data = include_bytes!("../assets/FiraCode-Regular.ttf");
        let font = Font::from_bytes(
            font_data.as_slice(),
            FontSettings {
                scale: 16.0 * scale_factor as f32,
                ..FontSettings::default()
            },
        ).map_err(|e| anyhow::anyhow!("Failed to load font: {}", e))?;

        let font_size = 16.0;
        let texture_atlas = TextureAtlas::new(&device, 1024, 1024);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_atlas.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let empty_vertices: &[Vertex] = &[];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(empty_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let empty_indices: &[u16] = &[];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(empty_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
            font,
            glyph_cache: HashMap::new(),
            texture_atlas,
            font_size,
            scale_factor,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            debug!("Resized to: {:?}", new_size);
        }
    }

    fn get_or_cache_glyph(&mut self, ch: char) -> Option<&GlyphInfo> {
        if !self.glyph_cache.contains_key(&ch) {
            let (metrics, bitmap) = self.font.rasterize(ch, self.font_size * self.scale_factor as f32);
            
            if metrics.width > 0 && metrics.height > 0 {
                if let Some(tex_coords) = self.texture_atlas.add_glyph(
                    &self.queue,
                    &bitmap,
                    metrics.width as u32,
                    metrics.height as u32,
                ) {
                    let glyph_info = GlyphInfo {
                        texture_coords: tex_coords,
                        size: [metrics.width as f32, metrics.height as f32],
                        bearing: [metrics.xmin as f32, metrics.ymin as f32],
                        advance: metrics.advance_width,
                    };
                    self.glyph_cache.insert(ch, glyph_info);
                }
            }
        }
        
        self.glyph_cache.get(&ch)
    }

    pub fn render(&mut self, terminal: &Terminal) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_count = 0u16;

        let cell_width = self.font_size * self.scale_factor as f32 * 0.6;
        let cell_height = self.font_size * self.scale_factor as f32 * 1.2;
        let screen_width = self.size.width as f32;
        let screen_height = self.size.height as f32;

        for (row_idx, row) in terminal.get_buffer().iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if let Some(glyph_info) = self.get_or_cache_glyph(cell.character) {
                    let x = col_idx as f32 * cell_width;
                    let y = row_idx as f32 * cell_height;

                    let screen_x = (x / screen_width) * 2.0 - 1.0;
                    let screen_y = 1.0 - (y / screen_height) * 2.0;
                    
                    let width = (glyph_info.size[0] / screen_width) * 2.0;
                    let height = (glyph_info.size[1] / screen_height) * 2.0;

                    let color = [
                        cell.foreground_color.0 as f32 / 255.0,
                        cell.foreground_color.1 as f32 / 255.0,
                        cell.foreground_color.2 as f32 / 255.0,
                        cell.foreground_color.3 as f32 / 255.0,
                    ];

                    vertices.extend_from_slice(&[
                        Vertex {
                            position: [screen_x, screen_y, 0.0],
                            tex_coords: [glyph_info.texture_coords[0], glyph_info.texture_coords[1]],
                            color,
                        },
                        Vertex {
                            position: [screen_x + width, screen_y, 0.0],
                            tex_coords: [glyph_info.texture_coords[2], glyph_info.texture_coords[1]],
                            color,
                        },
                        Vertex {
                            position: [screen_x + width, screen_y - height, 0.0],
                            tex_coords: [glyph_info.texture_coords[2], glyph_info.texture_coords[3]],
                            color,
                        },
                        Vertex {
                            position: [screen_x, screen_y - height, 0.0],
                            tex_coords: [glyph_info.texture_coords[0], glyph_info.texture_coords[3]],
                            color,
                        },
                    ]);

                    indices.extend_from_slice(&[
                        vertex_count,
                        vertex_count + 1,
                        vertex_count + 2,
                        vertex_count,
                        vertex_count + 2,
                        vertex_count + 3,
                    ]);

                    vertex_count += 4;
                }
            }
        }

        if !vertices.is_empty() {
            let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        } else {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}