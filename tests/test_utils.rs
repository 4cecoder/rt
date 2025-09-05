use rt::terminal::{Terminal, TerminalCell, Color};
use std::time::{Duration, Instant};

pub struct TestTimer {
    start: Instant,
    name: String,
}

impl TestTimer {
    pub fn new(name: &str) -> Self {
        Self {
            start: Instant::now(),
            name: name.to_string(),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    pub fn assert_under(&self, max_duration: Duration) {
        let elapsed = self.elapsed();
        assert!(
            elapsed <= max_duration,
            "{} took {:?}, expected <= {:?}",
            self.name, elapsed, max_duration
        );
    }
}

impl Drop for TestTimer {
    fn drop(&mut self) {
        println!("{} completed in {:?}", self.name, self.elapsed());
    }
}

pub struct TerminalTestBuilder {
    width: usize,
    height: usize,
    content: Option<String>,
}

impl TerminalTestBuilder {
    pub fn new() -> Self {
        Self {
            width: 80,
            height: 24,
            content: None,
        }
    }
    
    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }
    
    pub fn build(self) -> Terminal {
        let mut terminal = Terminal::with_size(self.width, self.height);
        
        if let Some(content) = self.content {
            terminal.write_text(&content);
        }
        
        terminal
    }
}

impl Default for TerminalTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryProfiler {
    terminal: Terminal,
    initial_memory: usize,
}

impl MemoryProfiler {
    pub fn new(terminal: Terminal) -> Self {
        let initial_memory = terminal.get_memory_usage();
        Self {
            terminal,
            initial_memory,
        }
    }
    
    pub fn current_memory(&self) -> usize {
        self.terminal.get_memory_usage()
    }
    
    pub fn memory_delta(&self) -> i64 {
        self.current_memory() as i64 - self.initial_memory as i64
    }
    
    pub fn memory_growth_mb(&self) -> f64 {
        self.memory_delta() as f64 / (1024.0 * 1024.0)
    }
    
    pub fn assert_memory_under(&self, max_mb: f64) {
        let current_mb = self.current_memory() as f64 / (1024.0 * 1024.0);
        assert!(
            current_mb <= max_mb,
            "Memory usage {:.2}MB exceeds limit {:.2}MB",
            current_mb, max_mb
        );
    }
    
    pub fn assert_growth_under(&self, max_growth_mb: f64) {
        let growth = self.memory_growth_mb();
        assert!(
            growth <= max_growth_mb,
            "Memory growth {:.2}MB exceeds limit {:.2}MB",
            growth, max_growth_mb
        );
    }
    
    pub fn terminal(&mut self) -> &mut Terminal {
        &mut self.terminal
    }
}

pub struct RenderingValidator {
    expected_chars: Vec<Vec<char>>,
    expected_colors: Vec<Vec<Color>>,
}

impl RenderingValidator {
    pub fn new() -> Self {
        Self {
            expected_chars: Vec::new(),
            expected_colors: Vec::new(),
        }
    }
    
    pub fn expect_line_chars(mut self, line: usize, chars: &str) -> Self {
        // Ensure we have enough lines
        while self.expected_chars.len() <= line {
            self.expected_chars.push(Vec::new());
        }
        
        self.expected_chars[line] = chars.chars().collect();
        self
    }
    
    pub fn expect_line_colors(mut self, line: usize, colors: Vec<Color>) -> Self {
        // Ensure we have enough lines
        while self.expected_colors.len() <= line {
            self.expected_colors.push(Vec::new());
        }
        
        self.expected_colors[line] = colors;
        self
    }
    
    pub fn validate(&self, terminal: &Terminal) {
        let buffer = terminal.get_buffer();
        
        // Validate characters
        for (line_idx, expected_line) in self.expected_chars.iter().enumerate() {
            if line_idx >= buffer.len() {
                panic!("Expected line {} but terminal only has {} lines", line_idx, buffer.len());
            }
            
            let actual_line = &buffer[line_idx];
            
            for (char_idx, expected_char) in expected_line.iter().enumerate() {
                if char_idx >= actual_line.len() {
                    panic!(
                        "Expected char at ({}, {}) but line only has {} chars",
                        line_idx, char_idx, actual_line.len()
                    );
                }
                
                let actual_char = actual_line[char_idx].character;
                assert_eq!(
                    actual_char, *expected_char,
                    "Character mismatch at ({}, {}): expected '{}', got '{}'",
                    line_idx, char_idx, expected_char, actual_char
                );
            }
        }
        
        // Validate colors
        for (line_idx, expected_colors) in self.expected_colors.iter().enumerate() {
            if line_idx >= buffer.len() {
                panic!("Expected color line {} but terminal only has {} lines", line_idx, buffer.len());
            }
            
            let actual_line = &buffer[line_idx];
            
            for (char_idx, expected_color) in expected_colors.iter().enumerate() {
                if char_idx >= actual_line.len() {
                    panic!(
                        "Expected color at ({}, {}) but line only has {} chars",
                        line_idx, char_idx, actual_line.len()
                    );
                }
                
                let actual_color = actual_line[char_idx].foreground_color;
                assert_eq!(
                    actual_color, *expected_color,
                    "Color mismatch at ({}, {}): expected {:?}, got {:?}",
                    line_idx, char_idx, expected_color, actual_color
                );
            }
        }
    }
}

impl Default for RenderingValidator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PerformanceTester {
    operations: Vec<Box<dyn Fn(&mut Terminal)>>,
    max_duration: Duration,
}

impl PerformanceTester {
    pub fn new(max_duration: Duration) -> Self {
        Self {
            operations: Vec::new(),
            max_duration,
        }
    }
    
    pub fn add_operation<F>(mut self, op: F) -> Self
    where
        F: Fn(&mut Terminal) + 'static,
    {
        self.operations.push(Box::new(op));
        self
    }
    
    pub fn run(&self, terminal: &mut Terminal) {
        let timer = TestTimer::new("PerformanceTester");
        
        for operation in &self.operations {
            operation(terminal);
        }
        
        timer.assert_under(self.max_duration);
    }
}

pub fn create_test_pattern(pattern_type: &str, size: usize) -> String {
    match pattern_type {
        "alphabet" => {
            (0..size)
                .map(|i| (b'A' + (i % 26) as u8) as char)
                .collect()
        }
        "numbers" => {
            (0..size)
                .map(|i| (b'0' + (i % 10) as u8) as char)
                .collect()
        }
        "operators" => {
            let ops = ['=', '!', '&', '|', '<', '>', '+', '-'];
            (0..size)
                .map(|i| ops[i % ops.len()])
                .collect()
        }
        "mixed" => {
            let chars = "ABC123=!&";
            (0..size)
                .map(|i| chars.chars().nth(i % chars.len()).unwrap())
                .collect()
        }
        "lines" => {
            (0..size)
                .map(|i| {
                    if i % 20 == 19 {
                        '\n'
                    } else {
                        (b'A' + ((i / 20) % 26) as u8) as char
                    }
                })
                .collect()
        }
        _ => "A".repeat(size),
    }
}

pub fn assert_terminal_state(
    terminal: &Terminal,
    expected_cursor: (usize, usize),
    expected_dimensions: (usize, usize),
) {
    assert_eq!(
        terminal.get_cursor_position(),
        expected_cursor,
        "Cursor position mismatch"
    );
    assert_eq!(
        terminal.get_dimensions(),
        expected_dimensions,
        "Terminal dimensions mismatch"
    );
}

pub fn fill_terminal_to_scroll(terminal: &mut Terminal, lines: usize) -> usize {
    let (_, height) = terminal.get_dimensions();
    let initial_history = terminal.get_history_size();
    
    for i in 0..lines {
        terminal.write_text(&format!("Scroll test line {}\n", i));
        
        // If we've filled the screen, further lines will cause scrolling
        if i >= height {
            assert!(terminal.get_history_size() > initial_history);
        }
    }
    
    terminal.get_history_size() - initial_history
}

pub fn benchmark_operation<F, R>(name: &str, operation: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = operation();
    let duration = start.elapsed();
    
    println!("Benchmark '{}' completed in {:?}", name, duration);
    
    (result, duration)
}

#[macro_export]
macro_rules! assert_memory_efficient {
    ($terminal:expr, $max_mb:expr) => {
        let memory_mb = $terminal.get_memory_usage() as f64 / (1024.0 * 1024.0);
        assert!(
            memory_mb <= $max_mb,
            "Memory usage {:.2}MB exceeds efficiency limit {:.2}MB",
            memory_mb, $max_mb
        );
    };
}

#[macro_export]
macro_rules! assert_performance {
    ($operation:expr, $max_duration:expr) => {{
        let start = std::time::Instant::now();
        let result = $operation;
        let duration = start.elapsed();
        assert!(
            duration <= $max_duration,
            "Operation took {:?}, expected <= {:?}",
            duration, $max_duration
        );
        result
    }};
}

#[cfg(test)]
mod test_utils_tests {
    use super::*;

    #[test]
    fn test_timer_functionality() {
        let timer = TestTimer::new("test");
        std::thread::sleep(Duration::from_millis(10));
        assert!(timer.elapsed() >= Duration::from_millis(10));
    }

    #[test]
    fn test_terminal_builder() {
        let terminal = TerminalTestBuilder::new()
            .with_size(100, 50)
            .with_content("Hello World")
            .build();
            
        assert_eq!(terminal.get_dimensions(), (100, 50));
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'H');
    }

    #[test]
    fn test_memory_profiler() {
        let terminal = Terminal::with_size(50, 25);
        let profiler = MemoryProfiler::new(terminal);
        
        assert!(profiler.current_memory() > 0);
        profiler.assert_memory_under(10.0); // Should be well under 10MB
    }

    #[test]
    fn test_rendering_validator() {
        let mut terminal = Terminal::new();
        terminal.write_text("ABC\n123");
        
        RenderingValidator::new()
            .expect_line_chars(0, "ABC")
            .expect_line_chars(1, "123")
            .validate(&terminal);
    }

    #[test]
    fn test_performance_tester() {
        let mut terminal = Terminal::with_size(20, 10);
        
        PerformanceTester::new(Duration::from_millis(100))
            .add_operation(|t| t.write_text("Hello"))
            .add_operation(|t| t.write_text(" World"))
            .add_operation(|t| t.clear())
            .run(&mut terminal);
    }

    #[test]
    fn test_pattern_generation() {
        assert_eq!(create_test_pattern("alphabet", 3), "ABC");
        assert_eq!(create_test_pattern("numbers", 3), "012");
        assert_eq!(create_test_pattern("operators", 2), "=!");
    }

    #[test]
    fn test_terminal_state_assertion() {
        let mut terminal = Terminal::with_size(80, 24);
        terminal.write_text("Hello");
        
        assert_terminal_state(&terminal, (5, 0), (80, 24));
    }

    #[test]
    fn test_scroll_filling() {
        let mut terminal = Terminal::with_size(10, 3);
        let scrolled_lines = fill_terminal_to_scroll(&mut terminal, 10);
        
        assert!(scrolled_lines > 0);
        assert!(terminal.get_history_size() > 0);
    }

    #[test]
    fn test_benchmark_operation() {
        let (result, duration) = benchmark_operation("simple_add", || 2 + 2);
        assert_eq!(result, 4);
        assert!(duration < Duration::from_secs(1));
    }
}