use std::collections::HashMap;
use std::fmt;
use vte::Parser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::new(0, 0, 0, 255);
    pub const RED: Color = Color::new(255, 0, 0, 255);
    pub const GREEN: Color = Color::new(0, 255, 0, 255);
    pub const YELLOW: Color = Color::new(255, 255, 0, 255);
    pub const BLUE: Color = Color::new(0, 0, 255, 255);
    pub const MAGENTA: Color = Color::new(255, 0, 255, 255);
    pub const CYAN: Color = Color::new(0, 255, 255, 255);
    pub const WHITE: Color = Color::new(255, 255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_ansi_color(color: u8) -> Self {
        match color {
            0 => Self::BLACK,
            1 => Self::RED,
            2 => Self::GREEN,
            3 => Self::YELLOW,
            4 => Self::BLUE,
            5 => Self::MAGENTA,
            6 => Self::CYAN,
            7 => Self::WHITE,
            8 => Color::new(128, 128, 128, 255), // Bright black (gray)
            9 => Color::new(255, 128, 128, 255), // Bright red
            10 => Color::new(128, 255, 128, 255), // Bright green
            11 => Color::new(255, 255, 128, 255), // Bright yellow
            12 => Color::new(128, 128, 255, 255), // Bright blue
            13 => Color::new(255, 128, 255, 255), // Bright magenta
            14 => Color::new(128, 255, 255, 255), // Bright cyan
            15 => Self::WHITE, // Bright white
            _ => Self::WHITE, // Default to white for unknown colors
        }
    }

    pub fn from_256_color(color: u8) -> Self {
        if color < 16 {
            return Self::from_ansi_color(color);
        }
        
        if color < 232 {
            // 216 colors (6x6x6 color cube)
            let index = color - 16;
            let r = (index / 36) * 51;
            let g = ((index % 36) / 6) * 51;
            let b = (index % 6) * 51;
            return Self::new(r, g, b, 255);
        }
        
        // 24 grayscale colors
        let gray = (color - 232) * 10 + 8;
        Self::new(gray, gray, gray, 255)
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub character: char,
    pub fg_color: Color,
    pub bg_color: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub reverse: bool,
    pub blink: bool,
    pub dim: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg_color: Color::WHITE,
            bg_color: Color::BLACK,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            reverse: false,
            blink: false,
            dim: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CursorState {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub style: CursorStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            visible: true,
            style: CursorStyle::Block,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub application_cursor: bool,
    pub origin_mode: bool,
    pub wraparound: bool,
    pub insert_mode: bool,
    pub auto_repeat: bool,
    pub newline_mode: bool,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub charset_g0: CharacterSet,
    pub charset_g1: CharacterSet,
    pub active_charset: usize,
    pub saved_cursor: CursorState,
    pub env_vars: HashMap<String, String>,
    pub working_directory: String,
}

#[derive(Debug, Clone, Copy)]
pub enum CharacterSet {
    Ascii,
    DecSpecial,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            application_cursor: false,
            origin_mode: false,
            wraparound: true,
            insert_mode: false,
            auto_repeat: true,
            newline_mode: false,
            scroll_top: 0,
            scroll_bottom: 0,
            charset_g0: CharacterSet::Ascii,
            charset_g1: CharacterSet::Ascii,
            active_charset: 0,
            saved_cursor: CursorState::default(),
            env_vars: HashMap::new(),
            working_directory: "/".to_string(),
        }
    }
}

pub struct Terminal {
    pub grid: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
    pub cursor: CursorState,
    pub state: TerminalState,
    pub parser: Parser,
    pub title: String,
    pub icon_title: String,
    pub tabs: Vec<usize>,
    pub current_fg: Color,
    pub current_bg: Color,
    pub current_attrs: CellAttributes,
}

#[derive(Debug, Default, Clone)]
pub struct CellAttributes {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub reverse: bool,
    pub blink: bool,
    pub dim: bool,
}

impl Terminal {
    pub fn new(width: usize, height: usize) -> Self {
        let mut grid = Vec::with_capacity(height);
        for _ in 0..height {
            grid.push(vec![Cell::default(); width]);
        }

        let mut state = TerminalState::default();
        state.scroll_bottom = height.saturating_sub(1);

        // Set default environment variables
        state.env_vars.insert("TERM".to_string(), "xterm-256color".to_string());
        state.env_vars.insert("COLORTERM".to_string(), "truecolor".to_string());

        let mut tabs = Vec::new();
        for i in (0..width).step_by(8) {
            tabs.push(i);
        }

        Self {
            grid,
            width,
            height,
            cursor: CursorState::default(),
            state,
            parser: Parser::new(),
            title: "rt - Terminal Emulator".to_string(),
            icon_title: "rt".to_string(),
            tabs,
            current_fg: Color::WHITE,
            current_bg: Color::BLACK,
            current_attrs: CellAttributes::default(),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        if width == self.width && height == self.height {
            return;
        }

        let old_width = self.width;
        let old_height = self.height;

        // Resize existing rows
        for row in &mut self.grid {
            if width > old_width {
                row.extend(vec![Cell::default(); width - old_width]);
            } else if width < old_width {
                row.truncate(width);
            }
        }

        // Add or remove rows
        if height > old_height {
            for _ in old_height..height {
                self.grid.push(vec![Cell::default(); width]);
            }
        } else if height < old_height {
            self.grid.truncate(height);
        }

        self.width = width;
        self.height = height;
        self.state.scroll_bottom = height.saturating_sub(1);

        // Adjust cursor position if necessary
        if self.cursor.x >= width {
            self.cursor.x = width.saturating_sub(1);
        }
        if self.cursor.y >= height {
            self.cursor.y = height.saturating_sub(1);
        }

        // Rebuild tab stops
        self.tabs.clear();
        for i in (0..width).step_by(8) {
            self.tabs.push(i);
        }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        for &byte in bytes {
            parser.advance(self, byte);
        }
        self.parser = parser;
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.grid.get(y)?.get(x)
    }

    pub fn get_cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        self.grid.get_mut(y)?.get_mut(x)
    }

    pub fn clear_screen(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    pub fn clear_line(&mut self, y: usize) {
        if let Some(row) = self.grid.get_mut(y) {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let start = self.state.scroll_top;
        let end = self.state.scroll_bottom.min(self.height.saturating_sub(1));
        
        for _ in 0..lines {
            if start < end {
                // Remove the top line and add a blank line at the bottom
                self.grid.remove(start);
                self.grid.insert(end, vec![Cell::default(); self.width]);
            }
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let start = self.state.scroll_top;
        let end = self.state.scroll_bottom.min(self.height.saturating_sub(1));
        
        for _ in 0..lines {
            if start < end {
                // Remove the bottom line and add a blank line at the top
                self.grid.remove(end);
                self.grid.insert(start, vec![Cell::default(); self.width]);
            }
        }
    }

    pub fn insert_lines(&mut self, count: usize) {
        let y = self.cursor.y;
        for _ in 0..count {
            if y <= self.state.scroll_bottom {
                if self.grid.len() > self.state.scroll_bottom {
                    self.grid.remove(self.state.scroll_bottom);
                }
                self.grid.insert(y, vec![Cell::default(); self.width]);
            }
        }
    }

    pub fn delete_lines(&mut self, count: usize) {
        let y = self.cursor.y;
        for _ in 0..count {
            if y <= self.state.scroll_bottom {
                if y < self.grid.len() {
                    self.grid.remove(y);
                }
                self.grid.insert(self.state.scroll_bottom, vec![Cell::default(); self.width]);
            }
        }
    }

    pub fn move_cursor_to(&mut self, x: usize, y: usize) {
        self.cursor.x = x.min(self.width.saturating_sub(1));
        self.cursor.y = if self.state.origin_mode {
            (self.state.scroll_top + y).min(self.state.scroll_bottom)
        } else {
            y.min(self.height.saturating_sub(1))
        };
    }

    pub fn move_cursor_relative(&mut self, dx: i32, dy: i32) {
        let new_x = (self.cursor.x as i32 + dx).max(0) as usize;
        let new_y = (self.cursor.y as i32 + dy).max(0) as usize;
        self.move_cursor_to(new_x, new_y);
    }

    pub fn set_environment_variable(&mut self, key: String, value: String) {
        self.state.env_vars.insert(key, value);
    }

    pub fn get_environment_variable(&self, key: &str) -> Option<&String> {
        self.state.env_vars.get(key)
    }

    pub fn set_working_directory(&mut self, path: String) {
        self.state.working_directory = path;
    }

    pub fn get_working_directory(&self) -> &str {
        &self.state.working_directory
    }
}

impl fmt::Display for Terminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (y, row) in self.grid.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if x == self.cursor.x && y == self.cursor.y && self.cursor.visible {
                    write!(f, "\x1b[7m{}\x1b[0m", cell.character)?;
                } else {
                    write!(f, "{}", cell.character)?;
                }
            }
            if y < self.grid.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}