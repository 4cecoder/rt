use anyhow::Result;
use log::info;
use std::time::Instant;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod renderer;
mod terminal;

use renderer::Renderer;
use terminal::Terminal;

fn main() -> Result<()> {
    env_logger::init();
    
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("RT - Advanced Terminal Emulator")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
        .build(&event_loop)?;

    let mut renderer = pollster::block_on(Renderer::new(&window))?;
    let mut terminal = Terminal::with_sample_data();
    
    let mut last_frame_time = Instant::now();
    let target_fps = 60.0;
    let target_frame_time = std::time::Duration::from_secs_f64(1.0 / target_fps);
    
    info!("Starting RT Terminal Emulator with advanced rendering engine");
    
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Close requested, shutting down");
                        elwt.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        renderer.resize(physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        let current_time = Instant::now();
                        let delta_time = current_time.duration_since(last_frame_time);
                        
                        if delta_time >= target_frame_time {
                            terminal.update(delta_time);
                            
                            match renderer.render(&terminal) {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    log::error!("Out of memory!");
                                    elwt.exit();
                                }
                                Err(e) => log::error!("Render error: {:?}", e),
                            }
                            
                            last_frame_time = current_time;
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        terminal.handle_keyboard_input(event);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        terminal.handle_mouse_input(state, button);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        terminal.handle_cursor_moved(position);
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        terminal.handle_scroll(delta);
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;
    
    Ok(())
}