use crate::terminal::{Terminal, Cell, Color};
use vte::{Params, Perform};

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        if c == '\0' {
            return;
        }

        // Handle character width
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        
        if self.cursor.x + char_width > self.width {
            if self.state.wraparound {
                self.cursor.x = 0;
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
            } else {
                self.cursor.x = self.width.saturating_sub(char_width);
            }
        }

        // Store values to avoid borrow checker issues
        let fg_color = if self.current_attrs.reverse { self.current_bg } else { self.current_fg };
        let bg_color = if self.current_attrs.reverse { self.current_fg } else { self.current_bg };
        let attrs = self.current_attrs.clone();

        if let Some(cell) = self.get_cell_mut(self.cursor.x, self.cursor.y) {
            cell.character = c;
            cell.fg_color = fg_color;
            cell.bg_color = bg_color;
            cell.bold = attrs.bold;
            cell.italic = attrs.italic;
            cell.underline = attrs.underline;
            cell.strikethrough = attrs.strikethrough;
            cell.reverse = attrs.reverse;
            cell.blink = attrs.blink;
            cell.dim = attrs.dim;
        }

        self.cursor.x += char_width;
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => {
                // BEL - Bell
                log::debug!("Bell character received");
            }
            0x08 => {
                // BS - Backspace
                if self.cursor.x > 0 {
                    self.cursor.x -= 1;
                }
            }
            0x09 => {
                // HT - Horizontal Tab
                let next_tab = self.tabs.iter().find(|&&tab| tab > self.cursor.x);
                if let Some(&next_tab) = next_tab {
                    self.cursor.x = next_tab.min(self.width.saturating_sub(1));
                } else {
                    self.cursor.x = self.width.saturating_sub(1);
                }
            }
            0x0A => {
                // LF - Line Feed
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
                if self.state.newline_mode {
                    self.cursor.x = 0;
                }
            }
            0x0B => {
                // VT - Vertical Tab (same as LF)
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
            }
            0x0C => {
                // FF - Form Feed (same as LF)
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
            }
            0x0D => {
                // CR - Carriage Return
                self.cursor.x = 0;
            }
            0x0E => {
                // SO - Shift Out (use G1 character set)
                self.state.active_charset = 1;
            }
            0x0F => {
                // SI - Shift In (use G0 character set)
                self.state.active_charset = 0;
            }
            _ => {
                log::debug!("Unhandled execute byte: 0x{:02X}", byte);
            }
        }
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        log::debug!("Hook: {:?} {:?} {} {:?}", params, intermediates, ignore, c);
    }

    fn put(&mut self, byte: u8) {
        log::debug!("Put: 0x{:02X}", byte);
    }

    fn unhook(&mut self) {
        log::debug!("Unhook");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }

        let param_str = String::from_utf8_lossy(params[0]);
        let parts: Vec<&str> = param_str.splitn(2, ';').collect();
        
        if parts.len() < 2 {
            return;
        }

        match parts[0] {
            "0" => {
                // Set icon name and window title
                self.icon_title = parts[1].to_string();
                self.title = parts[1].to_string();
            }
            "1" => {
                // Set icon name
                self.icon_title = parts[1].to_string();
            }
            "2" => {
                // Set window title
                self.title = parts[1].to_string();
            }
            "7" => {
                // Set working directory
                if let Some(path) = parts[1].strip_prefix("file://") {
                    self.set_working_directory(path.to_string());
                } else {
                    self.set_working_directory(parts[1].to_string());
                }
            }
            _ => {
                log::debug!("Unhandled OSC sequence: {}", param_str);
            }
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        if ignore {
            return;
        }

        match c {
            'A' => {
                // CUU - Cursor Up
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                let new_y = self.cursor.y.saturating_sub(count);
                self.cursor.y = if self.state.origin_mode {
                    new_y.max(self.state.scroll_top)
                } else {
                    new_y
                };
            }
            'B' => {
                // CUD - Cursor Down
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                let new_y = self.cursor.y + count;
                self.cursor.y = if self.state.origin_mode {
                    new_y.min(self.state.scroll_bottom)
                } else {
                    new_y.min(self.height.saturating_sub(1))
                };
            }
            'C' => {
                // CUF - Cursor Forward
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.x = (self.cursor.x + count).min(self.width.saturating_sub(1));
            }
            'D' => {
                // CUB - Cursor Backward
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.x = self.cursor.x.saturating_sub(count);
            }
            'E' => {
                // CNL - Cursor Next Line
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.x = 0;
                self.cursor.y = (self.cursor.y + count).min(self.height.saturating_sub(1));
            }
            'F' => {
                // CPL - Cursor Previous Line
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.x = 0;
                self.cursor.y = self.cursor.y.saturating_sub(count);
            }
            'G' => {
                // CHA - Cursor Horizontal Absolute
                let col = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.x = (col.saturating_sub(1)).min(self.width.saturating_sub(1));
            }
            'H' | 'f' => {
                // CUP - Cursor Position
                let row = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                let col = params.iter().nth(1).and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.move_cursor_to(col.saturating_sub(1), row.saturating_sub(1));
            }
            'I' => {
                // CHT - Cursor Forward Tabulation
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                for _ in 0..count {
                    if let Some(&next_tab) = self.tabs.iter().find(|&&tab| tab > self.cursor.x) {
                        self.cursor.x = next_tab.min(self.width.saturating_sub(1));
                    } else {
                        self.cursor.x = self.width.saturating_sub(1);
                        break;
                    }
                }
            }
            'J' => {
                // ED - Erase in Display
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                match mode {
                    0 => {
                        // Clear from cursor to end of screen
                        for y in self.cursor.y..self.height {
                            let start_x = if y == self.cursor.y { self.cursor.x } else { 0 };
                            if let Some(row) = self.grid.get_mut(y) {
                                for cell in &mut row[start_x..] {
                                    *cell = Cell::default();
                                }
                            }
                        }
                    }
                    1 => {
                        // Clear from beginning of screen to cursor
                        for y in 0..=self.cursor.y {
                            let end_x = if y == self.cursor.y { self.cursor.x + 1 } else { self.width };
                            if let Some(row) = self.grid.get_mut(y) {
                                for cell in &mut row[0..end_x.min(self.width)] {
                                    *cell = Cell::default();
                                }
                            }
                        }
                    }
                    2 | 3 => {
                        // Clear entire screen
                        self.clear_screen();
                        if mode == 3 {
                            // Also clear scrollback buffer (not implemented yet)
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                // EL - Erase in Line
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                if let Some(row) = self.grid.get_mut(self.cursor.y) {
                    match mode {
                        0 => {
                            // Clear from cursor to end of line
                            for cell in &mut row[self.cursor.x..] {
                                *cell = Cell::default();
                            }
                        }
                        1 => {
                            // Clear from beginning of line to cursor
                            for cell in &mut row[0..=self.cursor.x.min(self.width.saturating_sub(1))] {
                                *cell = Cell::default();
                            }
                        }
                        2 => {
                            // Clear entire line
                            for cell in row {
                                *cell = Cell::default();
                            }
                        }
                        _ => {}
                    }
                }
            }
            'L' => {
                // IL - Insert Line
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.insert_lines(count);
            }
            'M' => {
                // DL - Delete Line
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.delete_lines(count);
            }
            'P' => {
                // DCH - Delete Character
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                if let Some(row) = self.grid.get_mut(self.cursor.y) {
                    for _ in 0..count {
                        if self.cursor.x < row.len() {
                            row.remove(self.cursor.x);
                            row.push(Cell::default());
                        }
                    }
                }
            }
            'S' => {
                // SU - Scroll Up
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.scroll_up(count);
            }
            'T' => {
                // SD - Scroll Down
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.scroll_down(count);
            }
            'X' => {
                // ECH - Erase Character
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                if let Some(row) = self.grid.get_mut(self.cursor.y) {
                    let end = (self.cursor.x + count).min(self.width);
                    for cell in &mut row[self.cursor.x..end] {
                        *cell = Cell::default();
                    }
                }
            }
            'Z' => {
                // CBT - Cursor Backward Tabulation
                let count = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                for _ in 0..count {
                    if let Some(&prev_tab) = self.tabs.iter().rev().find(|&&tab| tab < self.cursor.x) {
                        self.cursor.x = prev_tab;
                    } else {
                        self.cursor.x = 0;
                        break;
                    }
                }
            }
            'd' => {
                // VPA - Vertical Position Absolute
                let row = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                self.cursor.y = (row.saturating_sub(1)).min(self.height.saturating_sub(1));
            }
            'g' => {
                // TBC - Tab Clear
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                match mode {
                    0 => {
                        // Clear tab at current position
                        self.tabs.retain(|&tab| tab != self.cursor.x);
                    }
                    3 => {
                        // Clear all tabs
                        self.tabs.clear();
                    }
                    _ => {}
                }
            }
            'h' => {
                // SM - Set Mode
                self.set_modes(params, true);
            }
            'l' => {
                // RM - Reset Mode
                self.set_modes(params, false);
            }
            'm' => {
                // SGR - Select Graphic Rendition
                self.handle_sgr(params);
            }
            'n' => {
                // DSR - Device Status Report
                let mode = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(0);
                match mode {
                    5 => {
                        // Device status - respond with "terminal OK"
                        log::debug!("Device status request - terminal OK");
                    }
                    6 => {
                        // Cursor position report
                        log::debug!("Cursor position: {}:{}", self.cursor.y + 1, self.cursor.x + 1);
                    }
                    _ => {}
                }
            }
            'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = params.iter().next().and_then(|p| p.first()).copied().unwrap_or(1) as usize;
                let bottom = params.iter().nth(1).and_then(|p| p.first()).copied().unwrap_or(self.height as u16) as usize;
                
                self.state.scroll_top = (top.saturating_sub(1)).min(self.height.saturating_sub(1));
                self.state.scroll_bottom = (bottom.saturating_sub(1)).min(self.height.saturating_sub(1));
                
                if self.state.scroll_top >= self.state.scroll_bottom {
                    self.state.scroll_top = 0;
                    self.state.scroll_bottom = self.height.saturating_sub(1);
                }
                
                // Move cursor to home position
                if self.state.origin_mode {
                    self.cursor.y = self.state.scroll_top;
                } else {
                    self.cursor.y = 0;
                }
                self.cursor.x = 0;
            }
            's' => {
                // DECSC - Save Cursor
                self.state.saved_cursor = self.cursor.clone();
            }
            'u' => {
                // DECRC - Restore Cursor
                self.cursor = self.state.saved_cursor.clone();
            }
            _ => {
                log::debug!("Unhandled CSI sequence: {:?} {:?} {}", params, intermediates, c);
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore {
            return;
        }

        match (intermediates.first().copied(), byte) {
            (None, b'7') => {
                // DECSC - Save Cursor
                self.state.saved_cursor = self.cursor.clone();
            }
            (None, b'8') => {
                // DECRC - Restore Cursor
                self.cursor = self.state.saved_cursor.clone();
            }
            (None, b'=') => {
                // DECKPAM - Keypad Application Mode
                self.state.application_cursor = true;
            }
            (None, b'>') => {
                // DECKPNM - Keypad Numeric Mode
                self.state.application_cursor = false;
            }
            (None, b'D') => {
                // IND - Index (move cursor down, scroll if at bottom)
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
            }
            (None, b'E') => {
                // NEL - Next Line
                self.cursor.x = 0;
                if self.cursor.y == self.state.scroll_bottom {
                    self.scroll_up(1);
                } else {
                    self.cursor.y += 1;
                }
            }
            (None, b'H') => {
                // HTS - Horizontal Tab Set
                if !self.tabs.contains(&self.cursor.x) {
                    self.tabs.push(self.cursor.x);
                    self.tabs.sort();
                }
            }
            (None, b'M') => {
                // RI - Reverse Index (move cursor up, scroll if at top)
                if self.cursor.y == self.state.scroll_top {
                    self.scroll_down(1);
                } else {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                }
            }
            (None, b'c') => {
                // RIS - Reset to Initial State
                *self = Terminal::new(self.width, self.height);
            }
            _ => {
                log::debug!("Unhandled ESC sequence: {:?} 0x{:02X}", intermediates, byte);
            }
        }
    }
}

impl Terminal {
    fn set_modes(&mut self, params: &Params, enable: bool) {
        for param_group in params.iter() {
            for &param in param_group {
                match param {
                    4 => self.state.insert_mode = enable,
                    20 => self.state.newline_mode = enable,
                    _ => {
                        log::debug!("Unhandled mode: {} ({})", param, if enable { "set" } else { "reset" });
                    }
                }
            }
        }
    }

    fn handle_sgr(&mut self, params: &Params) {
        if params.is_empty() {
            // Reset all attributes
            self.current_fg = Color::WHITE;
            self.current_bg = Color::BLACK;
            self.current_attrs = Default::default();
            return;
        }

        for param_group in params.iter() {
            for &param in param_group {
                match param {
                    0 => {
                        // Reset all attributes
                        self.current_fg = Color::WHITE;
                        self.current_bg = Color::BLACK;
                        self.current_attrs = Default::default();
                    }
                    1 => self.current_attrs.bold = true,
                    2 => self.current_attrs.dim = true,
                    3 => self.current_attrs.italic = true,
                    4 => self.current_attrs.underline = true,
                    5 => self.current_attrs.blink = true,
                    7 => self.current_attrs.reverse = true,
                    9 => self.current_attrs.strikethrough = true,
                    22 => {
                        self.current_attrs.bold = false;
                        self.current_attrs.dim = false;
                    }
                    23 => self.current_attrs.italic = false,
                    24 => self.current_attrs.underline = false,
                    25 => self.current_attrs.blink = false,
                    27 => self.current_attrs.reverse = false,
                    29 => self.current_attrs.strikethrough = false,
                    30..=37 => self.current_fg = Color::from_ansi_color((param - 30) as u8),
                    38 => {
                        // Set foreground color (256-color or RGB)
                        // This is a simplified implementation
                        // In a full implementation, you'd parse the next parameters
                    }
                    39 => self.current_fg = Color::WHITE,
                    40..=47 => self.current_bg = Color::from_ansi_color((param - 40) as u8),
                    48 => {
                        // Set background color (256-color or RGB)
                        // This is a simplified implementation
                    }
                    49 => self.current_bg = Color::BLACK,
                    90..=97 => self.current_fg = Color::from_ansi_color((param - 90 + 8) as u8),
                    100..=107 => self.current_bg = Color::from_ansi_color((param - 100 + 8) as u8),
                    _ => {
                        log::debug!("Unhandled SGR parameter: {}", param);
                    }
                }
            }
        }
    }
}