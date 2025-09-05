mod terminal;
mod ansi;
mod pty;
mod signals;
mod environment;

use terminal::Terminal;
use pty::{PtyConfig, PtyManager};
use signals::{SignalHandler, setup_panic_handler};
use environment::EnvironmentManager;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    env_logger::init();
    setup_panic_handler();

    info!("Starting RT Terminal Emulator v{}", env!("CARGO_PKG_VERSION"));

    // Initialize core components
    let mut terminal = Terminal::new(80, 24);
    let mut pty_manager = PtyManager::new();
    let signal_handler = SignalHandler::new();
    let env_manager = EnvironmentManager::new();

    // Setup signal handling
    signal_handler.setup_signal_handlers().await?;

    // Create initial PTY session with environment
    let mut pty_config = PtyConfig::default();
    pty_config.env_vars = env_manager.export_for_subprocess();
    pty_config.working_dir = env_manager.get_working_directory().to_string_lossy().to_string();
    pty_config.shell = env_manager.get_shell();

    let session_id = pty_manager.create_session(pty_config)?;
    info!("Created PTY session with ID: {}", session_id);

    // Get the PTY session
    let session = pty_manager.get_session(session_id)
        .ok_or("Failed to get PTY session")?;

    // Subscribe to PTY output
    let mut output_receiver = session.subscribe_output();

    // Main event loop
    loop {
        tokio::select! {
            // Handle PTY output
            output = output_receiver.recv() => {
                if let Ok(data) = output {
                    // Process output through terminal emulator
                    terminal.process_bytes(&data);
                    
                    // For demo purposes, print the terminal state
                    // In a real implementation, this would be rendered to a GUI
                    if log::log_enabled!(log::Level::Debug) {
                        for (y, row) in terminal.grid.iter().enumerate() {
                            let line: String = row.iter().map(|cell| cell.character).collect();
                            log::debug!("Line {}: '{}'", y, line.trim_end());
                        }
                    }
                } else {
                    info!("PTY output channel closed");
                    break;
                }
            }

            // Handle shutdown signals
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                if signal_handler.is_shutdown_requested() {
                    info!("Shutdown requested, cleaning up...");
                    break;
                }

                // Handle window resize
                if signal_handler.is_resize_requested() {
                    signal_handler.clear_resize_request();
                    info!("Window resize detected");
                    // In a real implementation, you would get the new size from the GUI
                    // and resize the terminal and PTY accordingly
                    // terminal.resize(new_width, new_height);
                    // session.resize(new_rows, new_cols);
                }

                // Cleanup dead sessions
                pty_manager.cleanup_dead_sessions();
            }
        }
    }

    // Cleanup
    if let Some(mut session) = pty_manager.remove_session(session_id) {
        let _ = session.kill();
    }

    info!("RT Terminal Emulator shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terminal_initialization() {
        let terminal = Terminal::new(80, 24);
        assert_eq!(terminal.width, 80);
        assert_eq!(terminal.height, 24);
        assert_eq!(terminal.cursor.x, 0);
        assert_eq!(terminal.cursor.y, 0);
    }

    #[tokio::test]
    async fn test_pty_manager_creation() {
        let mut manager = PtyManager::new();
        let config = PtyConfig::default();
        
        let session_id = manager.create_session(config).unwrap();
        assert!(manager.get_session(session_id).is_some());
        
        manager.cleanup_dead_sessions();
    }

    #[tokio::test]
    async fn test_environment_manager() {
        let env_manager = EnvironmentManager::new();
        assert!(env_manager.get_variable("TERM").is_some());
        assert!(env_manager.get_variable("COLORTERM").is_some());
    }

    #[test]
    fn test_signal_handler() {
        let handler = SignalHandler::new();
        assert!(!handler.is_shutdown_requested());
        assert!(!handler.is_resize_requested());
    }
}
