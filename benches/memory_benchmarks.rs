use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rt::terminal::{Terminal, TerminalCell, Color};
use std::time::Duration;

fn benchmark_memory_usage_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage_patterns");
    
    // Test memory usage for different terminal sizes
    for (width, height) in [(80, 24), (120, 40), (200, 60), (500, 200)].iter() {
        let buffer_size = width * height;
        group.throughput(Throughput::Elements(buffer_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("memory_footprint", format!("{}x{}", width, height)),
            &(*width, *height),
            |b, (w, h)| {
                b.iter(|| {
                    let terminal = Terminal::with_size(black_box(*w), black_box(*h));
                    black_box(terminal.get_memory_usage())
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation_patterns");
    
    // Test allocation efficiency for buffer creation
    group.bench_function("buffer_allocation", |b| {
        b.iter(|| {
            let terminal = Terminal::with_size(black_box(100), black_box(50));
            // Force memory calculation
            let memory = terminal.get_memory_usage();
            black_box(memory)
        })
    });
    
    // Test memory usage during heavy text input
    group.bench_function("text_input_memory_growth", |b| {
        b.iter_batched(
            || Terminal::with_size(80, 24),
            |mut terminal| {
                let initial_memory = terminal.get_memory_usage();
                
                // Add substantial text content
                for i in 0..black_box(500) {
                    terminal.write_text(&format!("Line {} with substantial content to test memory growth\n", i));
                }
                
                let final_memory = terminal.get_memory_usage();
                black_box(final_memory - initial_memory)
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    group.finish();
}

fn benchmark_history_buffer_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("history_buffer_management");
    
    // Test history buffer growth
    group.bench_function("history_accumulation", |b| {
        b.iter_batched(
            || Terminal::with_size(20, 5), // Small terminal to force scrolling
            |mut terminal| {
                let initial_history = terminal.get_history_size();
                
                // Force scrolling to build history
                for i in 0..black_box(100) {
                    terminal.write_text(&format!("Scroll line {}\n", i));
                }
                
                let final_history = terminal.get_history_size();
                black_box(final_history - initial_history)
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    // Test history access patterns
    group.bench_function("history_access", |b| {
        // Pre-create terminal with history
        let mut terminal = Terminal::with_size(20, 5);
        for i in 0..200 {
            terminal.write_text(&format!("History line {}\n", i));
        }
        
        b.iter(|| {
            let history_size = terminal.get_history_size();
            let mut total_chars = 0;
            
            // Access random history lines
            for i in 0..black_box(history_size.min(50)) {
                if let Some(line) = terminal.get_line_from_history(i) {
                    total_chars += line.len();
                }
            }
            
            black_box(total_chars)
        })
    });
    
    group.finish();
}

fn benchmark_cell_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cell_memory_efficiency");
    
    // Test individual cell operations
    group.bench_function("cell_creation", |b| {
        b.iter(|| {
            let cell = TerminalCell {
                character: black_box('A'),
                foreground_color: black_box(Color(255, 255, 255, 255)),
                background_color: black_box(Color(0, 0, 0, 255)),
                bold: black_box(false),
                italic: black_box(false),
                underline: black_box(false),
            };
            black_box(cell)
        })
    });
    
    // Test cell cloning efficiency
    group.bench_function("cell_cloning", |b| {
        let cell = TerminalCell {
            character: 'A',
            foreground_color: Color(200, 100, 50, 255),
            background_color: Color(10, 20, 30, 255),
            bold: true,
            italic: false,
            underline: true,
        };
        
        b.iter(|| {
            let cloned = black_box(cell.clone());
            black_box(cloned)
        })
    });
    
    // Test bulk cell operations
    group.bench_function("bulk_cell_operations", |b| {
        b.iter(|| {
            let mut cells = Vec::with_capacity(1000);
            
            for i in 0..black_box(1000) {
                let cell = TerminalCell {
                    character: (b'A' + (i % 26) as u8) as char,
                    foreground_color: Color((i % 255) as u8, 255, 255, 255),
                    ..Default::default()
                };
                cells.push(cell);
            }
            
            black_box(cells)
        })
    });
    
    group.finish();
}

fn benchmark_resize_memory_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_memory_impact");
    
    // Test memory changes during resize operations
    let resize_patterns = [
        ("double_size", (40, 20), (80, 40)),
        ("half_size", (80, 40), (40, 20)),
        ("aspect_change", (80, 20), (40, 40)),
        ("extreme_wide", (20, 40), (200, 10)),
    ];
    
    for (name, initial_size, new_size) in resize_patterns.iter() {
        group.bench_function(format!("{}_memory_delta", name), |b| {
            b.iter_batched(
                || {
                    let mut terminal = Terminal::with_size(initial_size.0, initial_size.1);
                    terminal.write_text("Sample content for memory testing during resize operations");
                    terminal
                },
                |mut terminal| {
                    let initial_memory = terminal.get_memory_usage();
                    terminal.resize(black_box(new_size.0), black_box(new_size.1));
                    let final_memory = terminal.get_memory_usage();
                    black_box(final_memory as i64 - initial_memory as i64)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    
    group.finish();
}

fn benchmark_memory_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_optimization");
    
    // Test memory optimization triggers
    group.bench_function("memory_calculation_overhead", |b| {
        let mut terminal = Terminal::with_size(100, 50);
        
        // Fill with content to make memory calculation more expensive
        for i in 0..200 {
            terminal.write_text(&format!("Line {} for memory calculation overhead test\n", i));
        }
        
        b.iter(|| {
            // This should trigger internal memory calculation
            let memory = terminal.get_memory_usage();
            black_box(memory)
        })
    });
    
    // Test update cycles with memory monitoring
    group.bench_function("update_with_memory_monitoring", |b| {
        let mut terminal = Terminal::with_size(80, 24);
        
        b.iter(|| {
            // Add some content
            terminal.write_text("Content");
            
            // Update (which includes memory monitoring)
            terminal.update(black_box(Duration::from_millis(16)));
            
            // Check memory usage
            black_box(terminal.get_memory_usage())
        })
    });
    
    group.finish();
}

fn benchmark_memory_fragmentation_resistance(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation_resistance");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));
    
    // Test repeated allocation and deallocation patterns
    group.bench_function("repeated_create_destroy", |b| {
        b.iter(|| {
            let mut terminals = Vec::new();
            
            // Create multiple terminals
            for i in 0..black_box(20) {
                let size = 50 + (i % 50);
                terminals.push(Terminal::with_size(size, size / 2));
            }
            
            // Access them to ensure they're not optimized away
            let total_memory: usize = terminals
                .iter()
                .map(|t| t.get_memory_usage())
                .sum();
            
            black_box(total_memory)
            // Drop happens automatically at end of scope
        })
    });
    
    // Test alternating large/small allocations
    group.bench_function("alternating_sizes", |b| {
        b.iter(|| {
            let mut terminals = Vec::new();
            
            for i in 0..black_box(10) {
                // Alternate between large and small terminals
                let terminal = if i % 2 == 0 {
                    Terminal::with_size(200, 100) // Large
                } else {
                    Terminal::with_size(20, 10)   // Small
                };
                terminals.push(terminal);
            }
            
            let total_memory: usize = terminals
                .iter()
                .map(|t| t.get_memory_usage())
                .sum();
            
            black_box(total_memory)
        })
    });
    
    group.finish();
}

fn benchmark_memory_under_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_under_load");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));
    
    // Test memory usage under sustained load
    group.bench_function("sustained_text_input", |b| {
        b.iter_batched(
            || Terminal::with_size(80, 30),
            |mut terminal| {
                let initial_memory = terminal.get_memory_usage();
                
                // Sustained text input for extended period
                for i in 0..black_box(2000) {
                    terminal.write_text(&format!("Sustained load test line {} with various content\n", i));
                    
                    // Occasional updates to simulate real usage
                    if i % 10 == 0 {
                        terminal.update(Duration::from_millis(16));
                    }
                }
                
                let final_memory = terminal.get_memory_usage();
                black_box(final_memory - initial_memory)
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    // Test memory under concurrent operations
    group.bench_function("concurrent_operations_memory", |b| {
        b.iter_batched(
            || Terminal::with_size(100, 40),
            |mut terminal| {
                let initial_memory = terminal.get_memory_usage();
                
                for i in 0..black_box(500) {
                    // Write text
                    terminal.write_text(&format!("Multi-op test {}\n", i));
                    
                    // Resize occasionally
                    if i % 20 == 0 {
                        let new_size = 80 + (i % 40);
                        terminal.resize(new_size, 40);
                    }
                    
                    // Update frequently
                    if i % 5 == 0 {
                        terminal.update(Duration::from_millis(16));
                    }
                }
                
                let final_memory = terminal.get_memory_usage();
                black_box(final_memory - initial_memory)
            },
            criterion::BatchSize::SmallInput,
        )
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_usage_patterns,
    benchmark_memory_allocation_patterns,
    benchmark_history_buffer_management,
    benchmark_cell_memory_efficiency,
    benchmark_resize_memory_impact,
    benchmark_memory_optimization,
    benchmark_memory_fragmentation_resistance,
    benchmark_memory_under_load
);

criterion_main!(benches);