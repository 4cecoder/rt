use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rt::terminal::{Terminal, TerminalCell, Color};
use std::time::Duration;
use winit::event::MouseScrollDelta;

fn benchmark_terminal_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal_creation");
    
    for (width, height) in [(80, 24), (120, 40), (200, 60), (500, 200)].iter() {
        group.throughput(Throughput::Elements((width * height) as u64));
        group.bench_with_input(
            BenchmarkId::new("create_terminal", format!("{}x{}", width, height)),
            &(*width, *height),
            |b, (w, h)| {
                b.iter(|| Terminal::with_size(black_box(*w), black_box(*h)))
            },
        );
    }
    group.finish();
}

fn benchmark_text_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_rendering");
    
    // Test different text lengths
    for text_length in [100, 1000, 5000, 10000].iter() {
        let text = "A".repeat(*text_length);
        group.throughput(Throughput::Elements(*text_length as u64));
        group.bench_with_input(
            BenchmarkId::new("write_text", text_length),
            &text,
            |b, text| {
                let mut terminal = Terminal::with_size(120, 50);
                b.iter(|| {
                    terminal.clear();
                    terminal.write_text(black_box(text))
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_character_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("character_rendering");
    
    // Test rendering of different character types
    let test_cases = [
        ("letters", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        ("numbers", "0123456789"),
        ("operators", "=!&|<>"),
        ("mixed", "Hello World! 123 => != && ||"),
    ];
    
    for (name, chars) in test_cases.iter() {
        group.bench_function(*name, |b| {
            let mut terminal = Terminal::with_size(80, 24);
            b.iter(|| {
                terminal.clear();
                for ch in chars.chars() {
                    terminal.write_char(black_box(ch));
                }
            })
        });
    }
    
    group.finish();
}

fn benchmark_color_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_processing");
    
    let color_test_text = "= 123 ABC != && || >= <= +++ ---";
    
    group.bench_function("color_assignment", |b| {
        let mut terminal = Terminal::with_size(80, 24);
        b.iter(|| {
            terminal.clear();
            terminal.write_text(black_box(color_test_text))
        })
    });
    
    group.finish();
}

fn benchmark_scroll_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("scroll_operations");
    
    group.bench_function("smooth_scrolling", |b| {
        let mut terminal = Terminal::with_size(80, 24);
        // Pre-fill with content
        for i in 0..100 {
            terminal.write_text(&format!("Line {} content\n", i));
        }
        
        b.iter(|| {
            let delta = MouseScrollDelta::LineDelta(0.0, black_box(3.0));
            terminal.handle_scroll(delta);
            for _ in 0..10 {
                terminal.update(black_box(Duration::from_millis(16)));
            }
        })
    });
    
    group.bench_function("scroll_up_with_history", |b| {
        let mut terminal = Terminal::with_size(20, 5);
        b.iter(|| {
            // Fill the terminal to trigger scrolling
            for i in 0..black_box(10) {
                terminal.write_text(&format!("Line {}\n", i));
            }
        })
    });
    
    group.finish();
}

fn benchmark_resize_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_operations");
    
    let resize_patterns = [
        ("enlarge", (40, 20), (80, 40)),
        ("shrink", (80, 40), (40, 20)),
        ("width_only", (80, 24), (120, 24)),
        ("height_only", (80, 24), (80, 40)),
    ];
    
    for (name, initial_size, new_size) in resize_patterns.iter() {
        group.bench_function(*name, |b| {
            b.iter_batched(
                || {
                    let mut terminal = Terminal::with_size(initial_size.0, initial_size.1);
                    terminal.write_text("Sample content that should be preserved during resize operations");
                    terminal
                },
                |mut terminal| {
                    terminal.resize(black_box(new_size.0), black_box(new_size.1))
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    
    group.finish();
}

fn benchmark_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    group.bench_function("memory_calculation", |b| {
        let mut terminal = Terminal::with_size(100, 50);
        b.iter(|| {
            // This will trigger memory calculation internally
            black_box(terminal.get_memory_usage())
        })
    });
    
    group.bench_function("cell_access", |b| {
        let terminal = Terminal::with_size(80, 24);
        b.iter(|| {
            // Random access pattern
            for x in 0..10 {
                for y in 0..5 {
                    black_box(terminal.get_cell_at(x, y));
                }
            }
        })
    });
    
    group.bench_function("cell_modification", |b| {
        let mut terminal = Terminal::with_size(80, 24);
        let test_cell = TerminalCell {
            character: 'X',
            foreground_color: Color(255, 0, 0, 255),
            ..Default::default()
        };
        
        b.iter(|| {
            // Random write pattern
            for x in 0..10 {
                for y in 0..5 {
                    black_box(terminal.set_cell_at(x, y, test_cell.clone()));
                }
            }
        })
    });
    
    group.finish();
}

fn benchmark_fps_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fps_simulation");
    group.measurement_time(Duration::from_secs(10));
    
    // Simulate 60 FPS rendering
    group.bench_function("60fps_update_cycle", |b| {
        let mut terminal = Terminal::with_sample_data();
        let frame_time = Duration::from_nanos(16_666_667); // ~60 FPS
        
        b.iter(|| {
            terminal.update(black_box(frame_time));
            // Simulate some text input during frame
            terminal.write_char(black_box('A'));
        })
    });
    
    // Simulate 120 FPS rendering
    group.bench_function("120fps_update_cycle", |b| {
        let mut terminal = Terminal::with_sample_data();
        let frame_time = Duration::from_nanos(8_333_333); // ~120 FPS
        
        b.iter(|| {
            terminal.update(black_box(frame_time));
            // Lighter workload for higher FPS
            black_box(terminal.get_cursor_position());
        })
    });
    
    group.finish();
}

fn benchmark_large_buffer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_buffer_operations");
    group.sample_size(10); // Fewer samples for large operations
    group.measurement_time(Duration::from_secs(15));
    
    // Test very large terminal buffers
    let sizes = [(500, 200), (1000, 100), (200, 500)];
    
    for (width, height) in sizes.iter() {
        group.bench_with_input(
            BenchmarkId::new("large_terminal_creation", format!("{}x{}", width, height)),
            &(*width, *height),
            |b, (w, h)| {
                b.iter(|| {
                    let terminal = Terminal::with_size(black_box(*w), black_box(*h));
                    black_box(terminal)
                })
            },
        );
    }
    
    // Test filling large buffers
    group.bench_function("fill_large_buffer", |b| {
        b.iter_batched(
            || Terminal::with_size(200, 100),
            |mut terminal| {
                for i in 0..1000 {
                    terminal.write_text(black_box(&format!("Line {} content\n", i)));
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    group.finish();
}

fn benchmark_stress_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_scenarios");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));
    
    // Rapid resize stress test
    group.bench_function("rapid_resize_stress", |b| {
        b.iter_batched(
            || {
                let mut terminal = Terminal::with_size(50, 25);
                terminal.write_text("Content to preserve during resizes");
                terminal
            },
            |mut terminal| {
                for i in 0..black_box(50) {
                    let size = 20 + (i % 60);
                    terminal.resize(size, size / 2);
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    // Rapid scroll stress test
    group.bench_function("rapid_scroll_stress", |b| {
        b.iter_batched(
            || {
                let mut terminal = Terminal::with_size(80, 24);
                // Pre-fill with content
                for i in 0..100 {
                    terminal.write_text(&format!("Line {}\n", i));
                }
                terminal
            },
            |mut terminal| {
                for i in 0..black_box(100) {
                    let delta = MouseScrollDelta::LineDelta(0.0, (i % 10) as f32);
                    terminal.handle_scroll(delta);
                    terminal.update(Duration::from_millis(1));
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    // Mixed operations stress test
    group.bench_function("mixed_operations_stress", |b| {
        b.iter_batched(
            || Terminal::with_size(80, 30),
            |mut terminal| {
                for i in 0..black_box(100) {
                    // Write some text
                    terminal.write_text(&format!("Content {}\n", i));
                    
                    // Occasional resize
                    if i % 10 == 0 {
                        terminal.resize(80 + (i % 40), 30 + (i % 20));
                    }
                    
                    // Occasional scroll
                    if i % 5 == 0 {
                        let delta = MouseScrollDelta::LineDelta(0.0, 2.0);
                        terminal.handle_scroll(delta);
                    }
                    
                    // Update animation
                    terminal.update(Duration::from_millis(16));
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_terminal_creation,
    benchmark_text_rendering,
    benchmark_character_rendering,
    benchmark_color_processing,
    benchmark_scroll_operations,
    benchmark_resize_operations,
    benchmark_memory_operations,
    benchmark_fps_simulation,
    benchmark_large_buffer_operations,
    benchmark_stress_scenarios
);

criterion_main!(benches);