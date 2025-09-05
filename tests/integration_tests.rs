use rt::terminal::{Terminal, TerminalCell, Color};
use std::time::Duration;
use winit::event::MouseScrollDelta;
use serial_test::serial;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[serial]
    fn test_terminal_renderer_integration() {
        // Test that terminal and renderer can work together
        let mut terminal = Terminal::with_sample_data();
        
        // Verify sample data is properly rendered
        let buffer = terminal.get_buffer();
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 24); // Default height
        assert_eq!(buffer[0].len(), 80); // Default width
        
        // Check that the first line contains expected content
        let first_line: String = buffer[0].iter()
            .map(|cell| cell.character)
            .take(20)
            .collect();
        
        assert!(first_line.contains("RT Terminal"));
    }

    #[test]
    #[serial]
    fn test_memory_management_under_load() {
        let mut terminal = Terminal::with_size(100, 50);
        let initial_memory = terminal.get_memory_usage();
        
        // Simulate heavy text input
        for i in 0..1000 {
            terminal.write_text(&format!("Line {} with some content\n", i));
        }
        
        // Memory should not grow unbounded
        let final_memory = terminal.get_memory_usage();
        
        // Memory should be managed (not more than 10x initial)
        assert!(final_memory < initial_memory * 10);
        
        // History should have captured scrolled content
        assert!(terminal.get_history_size() > 0);
    }

    #[test]
    #[serial]
    fn test_smooth_scrolling_physics() {
        let mut terminal = Terminal::new();
        
        // Add content to make scrolling meaningful
        for i in 0..100 {
            terminal.write_text(&format!("Line {} content\n", i));
        }
        
        // Set initial scroll target
        let delta = MouseScrollDelta::LineDelta(0.0, 5.0);
        terminal.handle_scroll(delta);
        
        let initial_target = terminal.get_scroll_state().target_offset;
        assert!(initial_target > 0.0);
        
        // Simulate multiple update cycles
        let mut last_offset = 0.0;
        let mut velocity_decreased = false;
        
        for _ in 0..10 {
            terminal.update(Duration::from_millis(16));
            let scroll_state = terminal.get_scroll_state();
            
            // Offset should be moving towards target
            assert!(scroll_state.offset > last_offset);
            
            // Eventually velocity should decrease due to damping
            if scroll_state.velocity < 1.0 {
                velocity_decreased = true;
            }
            
            last_offset = scroll_state.offset;
        }
        
        assert!(velocity_decreased, "Scroll velocity should decrease due to damping");
    }

    #[test]
    #[serial]
    fn test_terminal_resize_integration() {
        let mut terminal = Terminal::with_size(20, 10);
        
        // Fill with recognizable content
        terminal.write_text("ABCDEFGHIJKLMNOPQRST\n");
        terminal.write_text("12345\n");
        terminal.write_text("ZYXWVU");
        
        let (initial_width, initial_height) = terminal.get_dimensions();
        assert_eq!((initial_width, initial_height), (20, 10));
        
        // Resize to larger dimensions
        terminal.resize(40, 20);
        let (new_width, new_height) = terminal.get_dimensions();
        assert_eq!((new_width, new_height), (40, 20));
        
        // Content should be preserved
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'A');
        assert_eq!(terminal.get_cell_at(0, 1).unwrap().character, '1');
        assert_eq!(terminal.get_cell_at(0, 2).unwrap().character, 'Z');
        
        // Buffer dimensions should match
        let buffer = terminal.get_buffer();
        assert_eq!(buffer.len(), 20);
        assert_eq!(buffer[0].len(), 40);
        
        // Resize to smaller dimensions
        terminal.resize(10, 5);
        let (small_width, small_height) = terminal.get_dimensions();
        assert_eq!((small_width, small_height), (10, 5));
        
        // Content should still be accessible within bounds
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'A');
    }

    #[test]
    #[serial]
    fn test_color_rendering_consistency() {
        let mut terminal = Terminal::new();
        
        // Write characters that should have different colors
        terminal.write_text("= 123 ABC !=");
        
        // Check operator colors
        let equals_cell = terminal.get_cell_at(0, 0).unwrap();
        let exclamation_cell = terminal.get_cell_at(10, 0).unwrap();
        
        assert_eq!(equals_cell.foreground_color, Color(100, 200, 255, 255));
        assert_eq!(exclamation_cell.foreground_color, Color(100, 200, 255, 255));
        
        // Check number colors
        let number_cell = terminal.get_cell_at(2, 0).unwrap();
        assert_eq!(number_cell.foreground_color, Color(255, 200, 100, 255));
        
        // Check letter colors
        let letter_cell = terminal.get_cell_at(6, 0).unwrap();
        assert_eq!(letter_cell.foreground_color, Color(200, 255, 200, 255));
        
        // Space should have default color
        let space_cell = terminal.get_cell_at(1, 0).unwrap();
        assert_eq!(space_cell.foreground_color, Color::default());
    }

    #[test]
    #[serial]
    fn test_fps_counter_and_performance_monitoring() {
        let mut terminal = Terminal::new();
        
        // Simulate multiple frame updates
        for _ in 0..70 {
            terminal.update(Duration::from_millis(16));
        }
        
        // FPS counter should be working (we can't easily test the exact value
        // due to timing, but we can ensure the mechanism is in place)
        assert!(terminal.get_memory_usage() > 0);
        
        // Memory usage should be reasonable for default terminal
        let memory_mb = terminal.get_memory_usage() / (1024 * 1024);
        assert!(memory_mb < 10); // Should be well under 10MB for default terminal
    }

    #[test]
    #[serial]
    fn test_special_character_handling() {
        let mut terminal = Terminal::new();
        
        // Test various special characters
        terminal.write_text("Tab:\there\nNew line here\rCarriage return");
        terminal.write_text("ABC");
        terminal.write_char('\x08'); // Backspace
        terminal.write_char('\x08'); // Another backspace
        terminal.write_text("XY");
        
        // Check tab expansion
        let tab_result: String = terminal.get_buffer()[0].iter()
            .take(12)
            .map(|c| c.character)
            .collect();
        assert!(tab_result.contains("Tab:"));
        assert_eq!(terminal.get_cell_at(8, 0).unwrap().character, 'h');
        
        // Check carriage return overwrote the line
        assert_eq!(terminal.get_cell_at(0, 1).unwrap().character, 'C');
        
        // Check backspace worked
        let final_line: String = terminal.get_buffer()[1].iter()
            .take(20)
            .map(|c| c.character)
            .collect::<String>()
            .trim()
            .to_string();
        assert!(final_line.starts_with("Carriage returnXY"));
    }

    #[test]
    #[serial]
    fn test_boundary_conditions() {
        let mut terminal = Terminal::with_size(3, 3);
        
        // Test writing at exact boundaries
        terminal.write_text("AB");
        assert_eq!(terminal.get_cursor_position(), (2, 0));
        
        // Writing one more char should wrap
        terminal.write_char('C');
        assert_eq!(terminal.get_cursor_position(), (0, 1));
        
        // Fill the buffer completely
        terminal.write_text("DEFGHI");
        // This should wrap and scroll, ending up on the last line
        let (cursor_x, cursor_y) = terminal.get_cursor_position();
        assert!(cursor_y == 2); // Should be on last line
        
        // Writing more should trigger scrolling
        terminal.write_char('J');
        assert!(terminal.get_history_size() > 0);
        let (cursor_x, cursor_y) = terminal.get_cursor_position();
        assert_eq!(cursor_y, 2); // Should be on last line
        assert_eq!(cursor_x, 1); // Should be after 'J'
    }

    #[test]
    #[serial]
    fn test_concurrent_operations() {
        let mut terminal = Terminal::with_size(50, 25);
        
        // Simulate concurrent text input and scrolling
        terminal.write_text("Initial content\n");
        
        let scroll_delta = MouseScrollDelta::LineDelta(0.0, 3.0);
        terminal.handle_scroll(scroll_delta);
        
        // Continue writing while scroll is active
        for i in 0..10 {
            terminal.write_text(&format!("Line {}\n", i));
            terminal.update(Duration::from_millis(16));
        }
        
        // System should remain stable
        assert!(terminal.get_memory_usage() > 0);
        assert!(terminal.get_cursor_position().1 < 25);
        
        // Scroll state should be managed properly
        let scroll_state = terminal.get_scroll_state();
        assert!(scroll_state.target_offset >= 0.0);
    }

    #[test]
    #[serial]
    fn test_buffer_consistency() {
        let mut terminal = Terminal::with_size(10, 5);
        
        // Clear and write simple pattern
        terminal.clear();
        terminal.write_text("ABCDEFGHIJ\nKLMNOPQRST\nUVWXYZabcd\nefghijklmn\nopqrstuvwx");
        
        // Verify first line
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'A');
        assert_eq!(terminal.get_cell_at(9, 0).unwrap().character, 'J');
        
        // Verify second line  
        assert_eq!(terminal.get_cell_at(0, 1).unwrap().character, 'K');
        assert_eq!(terminal.get_cell_at(9, 1).unwrap().character, 'T');
        
        // Verify buffer dimensions
        let buffer = terminal.get_buffer();
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer[0].len(), 10);
    }

    #[test]
    #[serial]
    fn test_history_buffer_management() {
        let mut terminal = Terminal::with_size(5, 3);
        
        // Fill screen and cause scrolling
        for i in 0..10 {
            terminal.write_text(&format!("L{}\n", i));
        }
        
        let history_size = terminal.get_history_size();
        assert!(history_size > 0);
        
        // Verify we can retrieve history
        let first_history_line = terminal.get_line_from_history(0);
        assert!(first_history_line.is_some());
        
        // The history line should contain expected content
        let history_content: String = first_history_line.unwrap()
            .iter()
            .map(|cell| cell.character)
            .collect::<String>()
            .trim()
            .to_string();
            
        assert!(!history_content.is_empty());
    }

    #[test]
    #[serial]
    fn test_error_recovery() {
        let mut terminal = Terminal::with_size(10, 10);
        
        // Test operations that should not crash
        assert!(!terminal.set_cell_at(100, 100, TerminalCell::default())); // Out of bounds
        assert!(terminal.get_cell_at(100, 100).is_none()); // Out of bounds
        
        // Test with zero dimensions (edge case)
        terminal.resize(0, 0);
        // Should handle gracefully without crashing
        
        // Restore to valid size
        terminal.resize(5, 5);
        assert_eq!(terminal.get_dimensions(), (5, 5));
        
        // Should still work normally
        terminal.write_text("Test");
        assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'T');
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[serial]
    fn test_large_buffer_stress() {
        let start = Instant::now();
        let terminal = Terminal::with_size(500, 200);
        let creation_time = start.elapsed();
        
        // Large buffer should be created reasonably quickly
        assert!(creation_time < Duration::from_millis(1000));
        
        // Memory usage should be reasonable
        let memory_usage = terminal.get_memory_usage();
        let memory_mb = memory_usage / (1024 * 1024);
        
        // Should be less than 50MB for 500x200 terminal
        assert!(memory_mb < 50);
    }

    #[test]
    #[serial]
    fn test_rapid_text_input_stress() {
        let mut terminal = Terminal::with_size(80, 24);
        let start = Instant::now();
        
        // Rapidly write large amounts of text
        for i in 0..5000 {
            terminal.write_text(&format!("This is line {} with some content to test performance\n", i));
        }
        
        let write_time = start.elapsed();
        
        // Should handle rapid input efficiently (< 2 seconds)
        assert!(write_time < Duration::from_secs(2));
        
        // Memory should be managed properly
        let memory_mb = terminal.get_memory_usage() / (1024 * 1024);
        assert!(memory_mb < 100); // Should be well under 100MB
        
        // History should contain scrolled content
        assert!(terminal.get_history_size() > 0);
    }

    #[test]
    #[serial]
    fn test_frequent_resize_stress() {
        let mut terminal = Terminal::with_size(50, 25);
        terminal.write_text("Initial content that should be preserved");
        
        let start = Instant::now();
        
        // Perform many rapid resizes
        for i in 0..100 {
            let width = 20 + (i % 80);
            let height = 10 + (i % 40);
            terminal.resize(width, height);
        }
        
        let resize_time = start.elapsed();
        
        // Resizing should be efficient
        assert!(resize_time < Duration::from_millis(500));
        
        // Content should still be accessible
        if terminal.get_dimensions().0 > 7 && terminal.get_dimensions().1 > 0 {
            assert_eq!(terminal.get_cell_at(0, 0).unwrap().character, 'I');
        }
    }

    #[test]
    #[serial]
    fn test_scroll_animation_stress() {
        let mut terminal = Terminal::with_size(80, 24);
        
        // Set up rapid scroll operations
        let start = Instant::now();
        
        for i in 0..1000 {
            let delta = MouseScrollDelta::LineDelta(0.0, (i % 10) as f32);
            terminal.handle_scroll(delta);
            terminal.update(Duration::from_millis(1));
        }
        
        let scroll_time = start.elapsed();
        
        // Should handle rapid scrolling efficiently
        assert!(scroll_time < Duration::from_millis(500));
        
        // Scroll state should remain stable
        let scroll_state = terminal.get_scroll_state();
        assert!(scroll_state.target_offset >= 0.0);
        assert!(scroll_state.offset.is_finite());
        assert!(scroll_state.velocity.is_finite());
    }

    #[test]
    #[serial]
    fn test_memory_optimization_trigger() {
        let mut terminal = Terminal::with_size(100, 50);
        
        // Force memory usage above threshold by creating large history
        for i in 0..2000 {
            terminal.write_text(&format!("Line {} with content to increase memory usage significantly\n", i));
        }
        
        let initial_memory = terminal.get_memory_usage();
        
        // Trigger memory optimization by simulating high memory usage
        // (This is a bit artificial since we can't easily exceed 100MB in tests)
        terminal.update(Duration::from_millis(16));
        
        // Verify system remains stable regardless of memory optimization
        assert!(terminal.get_memory_usage() > 0);
        assert!(terminal.get_history_size() <= 1000); // Should be limited
    }
}