use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Default for Color {
    fn default() -> Self {
        Color(255, 255, 255, 255)
    }
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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
    history: Vec<Vec<TerminalCell>>, // Added for testing scrollback
}

impl Terminal {
    pub fn new() -> Self {
        Self::with_size(80, 24)
    }

    pub fn with_size(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(height);
        for _ in 0..height {
            let mut row = Vec::with_capacity(width);
            for _ in 0..width {
                row.push(TerminalCell::default());
            }
            buffer.push(row);
        }

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
            history: Vec::new(),
        };

        terminal.calculate_memory_usage();
        terminal
    }

    pub fn with_sample_data() -> Self {
        let sample_text = "RT Terminal Emulator - Advanced Rendering Engine\n\
                          Features: 60+ FPS, Hardware Acceleration, Ligatures\n\
                          Programming symbols: => != === >= <= && ||\n\
                          High-DPI support with efficient memory management\n\
                          Smooth scrolling and animations enabled\n\
                          \n\
                          Ready for intensive operations!";

        let mut terminal = Self::new();
        terminal.write_text(sample_text);
        terminal.calculate_memory_usage();
        terminal
    }

    pub fn get_buffer(&self) -> &Vec<Vec<TerminalCell>> {
        &self.buffer
    }

    pub fn get_cursor_position(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn get_scroll_state(&self) -> &ScrollState {
        &self.scroll_state
    }

    pub fn get_memory_usage(&self) -> usize {
        self.memory_usage
    }

    pub fn get_dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn clear(&mut self) {
        for row in &mut self.buffer {
            for cell in row {
                *cell = TerminalCell::default();
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.calculate_memory_usage();
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

    pub fn write_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.write_char(ch);
        }
    }

    pub fn write_char(&mut self, ch: char) {
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
            '\t' => {
                // Tab to next 8-character boundary
                let next_tab = ((self.cursor_x / 8) + 1) * 8;
                for _ in self.cursor_x..next_tab.min(self.width) {
                    self.write_char(' ');
                }
            }
            '\x08' => { // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                    self.buffer[self.cursor_y][self.cursor_x] = TerminalCell::default();
                }
            }
            c => {
                if self.cursor_x < self.width && self.cursor_y < self.height {
                    let color = self.get_color_for_char(c);
                    
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

    fn get_color_for_char(&self, c: char) -> Color {
        match c {
            '=' | '!' | '&' | '|' | '<' | '>' => Color(100, 200, 255, 255), // Blue for operators
            '0'..='9' => Color(255, 200, 100, 255), // Orange for numbers
            'A'..='Z' | 'a'..='z' => Color(200, 255, 200, 255), // Green for letters
            _ => Color::default(),
        }
    }

    fn scroll_up(&mut self) {
        // Save the first line to history
        if let Some(first_line) = self.buffer.first() {
            self.history.push(first_line.clone());
        }
        
        self.buffer.remove(0);
        let mut new_row = Vec::with_capacity(self.width);
        for _ in 0..self.width {
            new_row.push(TerminalCell::default());
        }
        self.buffer.push(new_row);
    }

    fn calculate_memory_usage(&mut self) {
        let cell_size = std::mem::size_of::<TerminalCell>();
        let buffer_size = if self.buffer.is_empty() {
            0
        } else {
            self.buffer.len() * self.buffer[0].len() * cell_size
        };
        let history_size = self.history.len() * self.width * cell_size;
        let overhead = std::mem::size_of::<Self>();
        self.memory_usage = buffer_size + history_size + overhead;
    }

    fn optimize_memory(&mut self) {
        log::info!("Optimizing memory usage...");
        
        const MAX_HISTORY_LINES: usize = 1000;
        if self.history.len() > MAX_HISTORY_LINES {
            let excess = self.history.len() - MAX_HISTORY_LINES;
            self.history.drain(0..excess);
        }
        
        self.calculate_memory_usage();
    }

    pub fn handle_keyboard_input(&mut self, event: KeyEvent) {
        if event.state == ElementState::Pressed {
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
        let max_scroll = (self.history.len() as f32 + self.buffer.len() as f32 - self.height as f32).max(0.0);
        self.scroll_state.target_offset = self.scroll_state.target_offset.min(max_scroll);
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        // Handle edge case of zero dimensions
        if new_width == 0 || new_height == 0 {
            return;
        }
        
        if new_width != self.width || new_height != self.height {
            log::info!("Resizing terminal: {}x{} -> {}x{}", 
                      self.width, self.height, new_width, new_height);
            
            let old_width = self.width;
            let old_height = self.height;
            
            self.width = new_width;
            self.height = new_height;
            
            // Handle width changes
            if new_width != old_width {
                for row in &mut self.buffer {
                    row.resize(new_width, TerminalCell::default());
                }
            }
            
            // Handle height changes
            if new_height > old_height {
                // Add new rows
                for _ in old_height..new_height {
                    let mut new_row = Vec::with_capacity(new_width);
                    for _ in 0..new_width {
                        new_row.push(TerminalCell::default());
                    }
                    self.buffer.push(new_row);
                }
            } else if new_height < old_height {
                // Remove excess rows
                self.buffer.truncate(new_height);
            }
            
            // Adjust cursor position if necessary
            self.cursor_x = self.cursor_x.min(new_width.saturating_sub(1));
            self.cursor_y = self.cursor_y.min(new_height.saturating_sub(1));
            
            self.calculate_memory_usage();
        }
    }

    pub fn get_cell_at(&self, x: usize, y: usize) -> Option<&TerminalCell> {
        self.buffer.get(y)?.get(x)
    }

    pub fn set_cell_at(&mut self, x: usize, y: usize, cell: TerminalCell) -> bool {
        if let Some(row) = self.buffer.get_mut(y) {
            if let Some(existing_cell) = row.get_mut(x) {
                *existing_cell = cell;
                return true;
            }
        }
        false
    }

    pub fn get_history_size(&self) -> usize {
        self.history.len()
    }

    pub fn get_line_from_history(&self, index: usize) -> Option<&Vec<TerminalCell>> {
        self.history.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use winit::event::MouseScrollDelta;

    #[test]
    fn test_terminal_creation() {
        let terminal = Terminal::new();
        assert_eq!(terminal.get_dimensions(), (80, 24));
        assert_eq!(terminal.get_cursor_position(), (0, 0));
        assert!(terminal.get_memory_usage() > 0);
    }

    #[test]
    fn test_terminal_with_custom_size() {
        let terminal = Terminal::with_size(120, 30);
        assert_eq!(terminal.get_dimensions(), (120, 30));
        assert_eq!(terminal.buffer.len(), 30);
        assert_eq!(terminal.buffer[0].len(), 120);
    }

    #[test]
    fn test_write_single_char() {
        let mut terminal = Terminal::new();
        terminal.write_char('A');
        
        assert_eq!(terminal.get_cursor_position(), (1, 0));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'A');
    }

    #[test]
    fn test_write_text() {
        let mut terminal = Terminal::new();
        terminal.write_text("Hello");
        
        assert_eq!(terminal.get_cursor_position(), (5, 0));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'H');
        assert_eq!(terminal.get_cell_at(4, 0).unwrap().character, 'o');
    }

    #[test]
    fn test_newline_handling() {
        let mut terminal = Terminal::new();
        terminal.write_text("Line1\nLine2");
        
        assert_eq!(terminal.get_cursor_position(), (5, 1));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'L');
        assert_eq!(terminal.get_cell_at(0, 1).unwrap().character, 'L');
    }

    #[test]
    fn test_carriage_return() {
        let mut terminal = Terminal::new();
        terminal.write_text("Hello\rWorld");
        
        assert_eq!(terminal.get_cursor_position(), (5, 0));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'W');
        assert_eq!(terminal.get_cell_at(4, 0).unwrap().character, 'd');
    }

    #[test]
    fn test_tab_handling() {
        let mut terminal = Terminal::new();
        terminal.write_text("A\tB");
        
        assert_eq!(terminal.get_cursor_position(), (9, 0));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'A');
        assert_eq!(terminal.get_cell_at(8, 0).unwrap().character, 'B');
    }

    #[test]
    fn test_backspace() {
        let mut terminal = Terminal::new();
        terminal.write_text("ABC");
        terminal.write_char('\x08'); // Backspace
        
        assert_eq!(terminal.get_cursor_position(), (2, 0));
        assert_eq!(terminal.get_cell_at(2, 0).unwrap().character, ' ');
    }

    #[test]
    fn test_line_wrapping() {
        let mut terminal = Terminal::with_size(5, 5);
        terminal.write_text("HelloWorld");
        
        assert_eq!(terminal.get_cursor_position(), (0, 2));
        assert_eq!(terminal.get_cell_at(4, 0).unwrap().character, 'o');
        assert_eq!(terminal.get_cell_at(0, 1).unwrap().character, 'W');
    }

    #[test]
    fn test_scroll_up() {
        let mut terminal = Terminal::with_size(10, 3);
        
        // Fill all lines and force an extra scroll
        terminal.write_text("Line1\nLine2\nLine3\nLine4");
        
        // Should have scrolled at least once
        assert!(terminal.get_history_size() > 0);
        
        // Should be on the last line
        let (cursor_x, cursor_y) = terminal.get_cursor_position();
        assert_eq!(cursor_y, 2); // Should be on last line
        assert_eq!(cursor_x, 5); // Should be after "Line4"
    }

    #[test]
    fn test_color_assignment() {
        let mut terminal = Terminal::new();
        terminal.write_char('=');
        terminal.write_char('5');
        terminal.write_char('A');
        
        let operator_cell = terminal.get_cell_at(0, 0).unwrap();
        let number_cell = terminal.get_cell_at(1, 0).unwrap();
        let letter_cell = terminal.get_cell_at(2, 0).unwrap();
        
        assert_eq!(operator_cell.foreground_color, Color(100, 200, 255, 255));
        assert_eq!(number_cell.foreground_color, Color(255, 200, 100, 255));
        assert_eq!(letter_cell.foreground_color, Color(200, 255, 200, 255));
    }

    #[test]
    fn test_clear() {
        let mut terminal = Terminal::new();
        terminal.write_text("Hello World");
        terminal.clear();
        
        assert_eq!(terminal.get_cursor_position(), (0, 0));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, ' ');
    }

    #[test]
    fn test_resize_larger() {
        let mut terminal = Terminal::with_size(5, 3);
        terminal.write_text("Hello");
        terminal.resize(10, 5);
        
        assert_eq!(terminal.get_dimensions(), (10, 5));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'H');
        assert_eq!(terminal.buffer.len(), 5);
        assert_eq!(terminal.buffer[0].len(), 10);
    }

    #[test]
    fn test_resize_smaller() {
        let mut terminal = Terminal::with_size(10, 5);
        terminal.write_text("Hello World");
        terminal.resize(5, 3);
        
        assert_eq!(terminal.get_dimensions(), (5, 3));
        assert_eq!(terminal.buffer.len(), 3);
        assert_eq!(terminal.buffer[0].len(), 5);
    }

    #[test]
    fn test_cursor_position_after_resize() {
        let mut terminal = Terminal::with_size(10, 5);
        terminal.cursor_x = 8;
        terminal.cursor_y = 4;
        terminal.resize(5, 3);
        
        let (x, y) = terminal.get_cursor_position();
        assert!(x < 5);
        assert!(y < 3);
    }

    #[test]
    fn test_memory_usage_calculation() {
        let terminal = Terminal::with_size(80, 24);
        let memory_usage = terminal.get_memory_usage();
        
        assert!(memory_usage > 0);
        
        let expected_min = std::mem::size_of::<TerminalCell>() * 80 * 24;
        assert!(memory_usage >= expected_min);
    }

    #[test]
    fn test_scroll_state_default() {
        let terminal = Terminal::new();
        let scroll_state = terminal.get_scroll_state();
        
        assert_eq!(scroll_state.offset, 0.0);
        assert_eq!(scroll_state.velocity, 0.0);
        assert_eq!(scroll_state.target_offset, 0.0);
    }

    #[test]
    fn test_handle_scroll() {
        let mut terminal = Terminal::new();
        // Add some content to make scrolling meaningful
        for i in 0..50 {
            terminal.write_text(&format!("Line {}\n", i));
        }
        
        let delta = MouseScrollDelta::LineDelta(0.0, 2.0);
        terminal.handle_scroll(delta);
        
        assert_eq!(terminal.scroll_state.target_offset, 6.0); // 2.0 * 3.0
    }

    #[test]
    fn test_scroll_clamping() {
        let mut terminal = Terminal::with_size(5, 3);
        let delta = MouseScrollDelta::LineDelta(0.0, -10.0);
        terminal.handle_scroll(delta);
        
        assert!(terminal.scroll_state.target_offset >= 0.0);
    }

    #[test]
    fn test_update_scroll_animation() {
        let mut terminal = Terminal::new();
        terminal.scroll_state.target_offset = 10.0;
        
        let delta_time = Duration::from_millis(16);
        terminal.update(delta_time);
        
        assert!(terminal.scroll_state.offset > 0.0);
        assert!(terminal.scroll_state.velocity != 0.0);
    }

    #[test]
    fn test_set_cell_at() {
        let mut terminal = Terminal::new();
        let custom_cell = TerminalCell {
            character: 'X',
            foreground_color: Color(255, 0, 0, 255),
            ..Default::default()
        };
        
        assert!(terminal.set_cell_at(5, 5, custom_cell.clone()));
        assert_eq!(terminal.get_cell_at(5, 5).unwrap(), &custom_cell);
    }

    #[test]
    fn test_set_cell_at_out_of_bounds() {
        let mut terminal = Terminal::with_size(10, 10);
        let custom_cell = TerminalCell::default();
        
        assert!(!terminal.set_cell_at(15, 15, custom_cell));
    }

    #[test]
    fn test_history_management() {
        let mut terminal = Terminal::with_size(5, 2);
        
        terminal.write_text("Line1\nLine2\nLine3");
        
        assert!(terminal.get_history_size() > 0);
        assert!(terminal.get_line_from_history(0).is_some());
    }

    #[test]
    fn test_sample_data_creation() {
        let terminal = Terminal::with_sample_data();
        
        // Check that sample data was written
        assert_ne!(terminal.get_cell_at(0, 0).unwrap().character, ' ');
        assert!(terminal.get_memory_usage() > 0);
    }

    #[test]
    fn test_large_buffer_performance() {
        let start = std::time::Instant::now();
        let terminal = Terminal::with_size(200, 100);
        let creation_time = start.elapsed();
        
        // Should create large terminal quickly (< 100ms)
        assert!(creation_time < Duration::from_millis(100));
        assert_eq!(terminal.get_dimensions(), (200, 100));
    }

    #[test]
    fn test_extensive_text_writing() {
        let mut terminal = Terminal::with_size(80, 24);
        let long_text = "A".repeat(10000);
        
        let start = std::time::Instant::now();
        terminal.write_text(&long_text);
        let write_time = start.elapsed();
        
        // Should handle large text input efficiently
        assert!(write_time < Duration::from_millis(500));
        assert!(terminal.get_history_size() > 0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_terminal_size_invariant(
            width in 1usize..=1000,
            height in 1usize..=1000
        ) {
            let terminal = Terminal::with_size(width, height);
            prop_assert_eq!(terminal.get_dimensions(), (width, height));
            prop_assert_eq!(terminal.buffer.len(), height);
            prop_assert_eq!(terminal.buffer[0].len(), width);
        }

        #[test]
        fn test_write_char_cursor_movement(c in any::<char>()) {
            let mut terminal = Terminal::with_size(10, 10);
            let initial_pos = terminal.get_cursor_position();
            
            terminal.write_char(c);
            let final_pos = terminal.get_cursor_position();
            
            match c {
                '\n' => {
                    prop_assert_eq!(final_pos.0, 0);
                    prop_assert_eq!(final_pos.1, initial_pos.1 + 1);
                }
                '\r' => {
                    prop_assert_eq!(final_pos.0, 0);
                    prop_assert_eq!(final_pos.1, initial_pos.1);
                }
                _ => {
                    // For normal characters, cursor should advance
                    prop_assert!(final_pos.0 > initial_pos.0 || final_pos.1 > initial_pos.1);
                }
            }
        }

        #[test]
        fn test_memory_usage_positive(
            width in 1usize..=100,
            height in 1usize..=100
        ) {
            let terminal = Terminal::with_size(width, height);
            prop_assert!(terminal.get_memory_usage() > 0);
        }

        #[test]
        fn test_resize_preserves_content(
            initial_width in 10usize..=50,
            initial_height in 10usize..=50,
            new_width in 10usize..=100,
            new_height in 10usize..=100
        ) {
            let mut terminal = Terminal::with_size(initial_width, initial_height);
            terminal.write_text("TEST");
            
            let original_char = terminal.get_cell_at(0, 0).unwrap().character;
            terminal.resize(new_width, new_height);
            
            prop_assert_eq!(terminal.get_dimensions(), (new_width, new_height));
            
            // Content should be preserved if within bounds
            if new_width > 0 && new_height > 0 {
                prop_assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, original_char);
            }
        }

        #[test]
        fn test_scroll_offset_bounds(
            scroll_lines in -100f32..=100f32
        ) {
            let mut terminal = Terminal::with_size(10, 5);
            let delta = MouseScrollDelta::LineDelta(0.0, scroll_lines);
            
            terminal.handle_scroll(delta);
            
            prop_assert!(terminal.scroll_state.target_offset >= 0.0);
        }
    }
}