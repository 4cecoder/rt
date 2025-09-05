use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};

#[derive(Clone, Copy, Debug)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Default for Color {
    fn default() -> Self {
        Color(255, 255, 255, 255)
    }
}

#[derive(Clone, Debug)]
pub struct TerminalCell {
    pub character: char,
    pub foreground_color: Color,
    pub background_color: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            character: ' ',
            foreground_color: Color::default(),
            background_color: Color(0, 0, 0, 255),
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

pub struct ScrollState {
    pub offset: f32,
    pub velocity: f32,
    pub target_offset: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            offset: 0.0,
            velocity: 0.0,
            target_offset: 0.0,
        }
    }
}

pub struct Terminal {
    buffer: Vec<Vec<TerminalCell>>,
    width: usize,
    height: usize,
    cursor_x: usize,
    cursor_y: usize,
    scroll_state: ScrollState,
    memory_usage: usize,
    frame_time: Duration,
    fps_counter: u32,
    last_fps_time: std::time::Instant,
}

impl Terminal {
    pub fn new() -> Self {
        let width = 80;
        let height = 24;
        
        let mut buffer = Vec::with_capacity(height);
        for _ in 0..height {
            let mut row = Vec::with_capacity(width);
            for _ in 0..width {
                row.push(TerminalCell::default());
            }
            buffer.push(row);
        }

        // Add some sample text to demonstrate rendering
        let sample_text = "RT Terminal Emulator - Advanced Rendering Engine\n\
                          Features: 60+ FPS, Hardware Acceleration, Ligatures\n\
                          Programming symbols: => != === >= <= && ||\n\
                          High-DPI support with efficient memory management\n\
                          Smooth scrolling and animations enabled\n\
                          \n\
                          Ready for intensive operations!";

        let mut terminal = Self {
            buffer,
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            scroll_state: ScrollState::default(),
            memory_usage: 0,
            frame_time: Duration::from_millis(16),
            fps_counter: 0,
            last_fps_time: std::time::Instant::now(),
        };

        terminal.write_text(sample_text);
        terminal.calculate_memory_usage();
        
        terminal
    }

    pub fn get_buffer(&self) -> &Vec<Vec<TerminalCell>> {
        &self.buffer
    }

    pub fn update(&mut self, delta_time: Duration) {
        self.frame_time = delta_time;
        self.fps_counter += 1;
        
        if self.last_fps_time.elapsed() >= Duration::from_secs(1) {
            log::debug!("FPS: {}, Memory usage: {} MB", 
                       self.fps_counter, 
                       self.memory_usage / (1024 * 1024));
            self.fps_counter = 0;
            self.last_fps_time = std::time::Instant::now();
        }

        // Smooth scrolling animation
        let scroll_speed = 8.0;
        let damping = 0.9;
        
        let diff = self.scroll_state.target_offset - self.scroll_state.offset;
        self.scroll_state.velocity += diff * scroll_speed * delta_time.as_secs_f32();
        self.scroll_state.velocity *= damping;
        self.scroll_state.offset += self.scroll_state.velocity * delta_time.as_secs_f32();

        // Ensure efficient memory usage
        if self.memory_usage > 100 * 1024 * 1024 {
            self.optimize_memory();
        }
    }

    fn write_text(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\n' => {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                    if self.cursor_y >= self.height {
                        self.scroll_up();
                        self.cursor_y = self.height - 1;
                    }
                }
                '\r' => {
                    self.cursor_x = 0;
                }
                c => {
                    if self.cursor_x < self.width && self.cursor_y < self.height {
                        let color = match c {
                            '=' | '!' | '&' | '|' | '<' | '>' => Color(100, 200, 255, 255), // Blue for operators
                            '0'..='9' => Color(255, 200, 100, 255), // Orange for numbers
                            'A'..='Z' | 'a'..='z' => Color(200, 255, 200, 255), // Green for letters
                            _ => Color::default(),
                        };
                        
                        self.buffer[self.cursor_y][self.cursor_x] = TerminalCell {
                            character: c,
                            foreground_color: color,
                            ..Default::default()
                        };
                        self.cursor_x += 1;
                        if self.cursor_x >= self.width {
                            self.cursor_x = 0;
                            self.cursor_y += 1;
                            if self.cursor_y >= self.height {
                                self.scroll_up();
                                self.cursor_y = self.height - 1;
                            }
                        }
                    }
                }
            }
        }
    }

    fn scroll_up(&mut self) {
        self.buffer.remove(0);
        let mut new_row = Vec::with_capacity(self.width);
        for _ in 0..self.width {
            new_row.push(TerminalCell::default());
        }
        self.buffer.push(new_row);
    }

    fn calculate_memory_usage(&mut self) {
        let cell_size = std::mem::size_of::<TerminalCell>();
        let buffer_size = self.buffer.len() * self.buffer[0].len() * cell_size;
        let overhead = std::mem::size_of::<Self>();
        self.memory_usage = buffer_size + overhead;
    }

    fn optimize_memory(&mut self) {
        // Implement memory optimization strategies
        log::info!("Optimizing memory usage...");
        
        // Example: Limit history buffer size
        const MAX_LINES: usize = 1000;
        if self.buffer.len() > MAX_LINES {
            let excess = self.buffer.len() - MAX_LINES;
            self.buffer.drain(0..excess);
        }
        
        self.calculate_memory_usage();
    }

    pub fn handle_keyboard_input(&mut self, event: KeyEvent) {
        if event.state == ElementState::Pressed {
            // Handle keyboard input
            log::debug!("Key pressed: {:?}", event.logical_key);
        }
    }

    pub fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        log::debug!("Mouse button {:?} {:?}", button, state);
    }

    pub fn handle_cursor_moved(&mut self, _position: PhysicalPosition<f64>) {
        // Handle cursor movement for potential text selection
    }

    pub fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => {
                self.scroll_state.target_offset += y * 3.0; // 3 lines per scroll
            }
            MouseScrollDelta::PixelDelta(position) => {
                self.scroll_state.target_offset += position.y as f32 * 0.1;
            }
        }
        
        // Clamp scroll offset
        self.scroll_state.target_offset = self.scroll_state.target_offset.max(0.0);
        let max_scroll = (self.buffer.len() as f32 - self.height as f32).max(0.0);
        self.scroll_state.target_offset = self.scroll_state.target_offset.min(max_scroll);
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if new_width != self.width || new_height != self.height {
            log::info!("Resizing terminal: {}x{} -> {}x{}", 
                      self.width, self.height, new_width, new_height);
            
            // Implement terminal resize logic
            self.width = new_width;
            self.height = new_height;
            
            // Adjust buffer size
            self.buffer.truncate(new_height);
            while self.buffer.len() < new_height {
                let mut row = Vec::with_capacity(new_width);
                for _ in 0..new_width {
                    row.push(TerminalCell::default());
                }
                self.buffer.push(row);
            }
            
            for row in &mut self.buffer {
                row.truncate(new_width);
                while row.len() < new_width {
                    row.push(TerminalCell::default());
                }
            }
            
            self.calculate_memory_usage();
        }
    }
}