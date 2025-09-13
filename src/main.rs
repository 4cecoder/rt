use std::time::Instant;
use wgpu::{Instance, InstanceDescriptor, RequestAdapterOptions, AdapterOptions, DeviceDescriptor, Features, Limits, SurfaceConfiguration, PresentMode, TextureFormat, TextureUsages, TextureViewDescriptor, CommandEncoderDescriptor, RenderPassColorAttachment, RenderPassDescriptor, Operations, Color};
use winit::{event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}, event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Window}};
use fontdue::{Font, FontSettings};
use image::{ImageBuffer, Rgba};
use tokio::runtime::Runtime;

mod terminal;
mod renderer;
mod input;

use terminal::{Terminal, TerminalBuffer};
use renderer::{Renderer, TextRenderer};
use input::InputHandler;

const WINDOW_WIDTH: u32 = 1200;
const WINDOW_HEIGHT: u32 = 800;
const FONT_SIZE: f32 = 16.0;
const TERMINAL_COLS: usize = 80;
const TERMINAL_ROWS: usize = 24;

struct AppState {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    renderer: Renderer,
    terminal: Terminal,
    input_handler: InputHandler,
    last_frame_time: Instant,
}

impl AppState {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();
        
        // Initialize WGPU
        let instance = Instance::new(InstanceDescriptor::default());
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        
        let adapter = instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();
        
        let (device, queue) = adapter.request_device(&DeviceDescriptor {
            label: None,
            features: Features::empty(),
            limits: Limits::default(),
        }, None).await.unwrap();
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        
        surface.configure(&device, &config);
        
        // Initialize renderer
        let renderer = Renderer::new(&device, &config);
        
        // Initialize terminal
        let terminal = Terminal::new(TERMINAL_COLS, TERMINAL_ROWS);
        
        // Initialize input handler
        let input_handler = InputHandler::new();
        
        Self {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            terminal,
            input_handler,
            last_frame_time: Instant::now(),
        }
    }
    
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = self.input_handler.handle_keyboard_input(input) {
                    self.terminal.handle_key_input(key);
                    true
                } else {
                    false
                }
            }
            WindowEvent::ReceivedCharacter(char) => {
                self.terminal.handle_char_input(*char);
                true
            }
            _ => false,
        }
    }
    
    fn update(&mut self) {
        // Update terminal state
        self.terminal.update();
        
        // Calculate frame time for performance monitoring
        let frame_time = self.last_frame_time.elapsed();
        self.last_frame_time = Instant::now();
        
        // Log performance metrics (in a real app, this would be more sophisticated)
        if frame_time.as_millis() > 16 {
            println!("Frame time: {}ms (target: 16ms)", frame_time.as_millis());
        }
    }
    
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            // Render terminal content
            self.renderer.render_terminal(&mut render_pass, &self.terminal);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Initialize Tokio runtime for async operations
    let runtime = Runtime::new().unwrap();
    
    // Initialize window
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("RT Terminal")
        .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build(&event_loop)
        .unwrap();
    
    // Initialize application state
    let mut state = AppState::new(window).await;
    
    // Main event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::MainEventsCleared => {
                state.update();
            }
            Event::RedrawRequested(window_id) if window_id == state.window.id() => {
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        state.resize(state.window.inner_size());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Err(e) => {
                        eprintln!("Render error: {:?}", e);
                    }
                }
            }
            _ => {}
        }
        
        // Request redraw for next frame
        state.window.request_redraw();
    }).unwrap();
}
