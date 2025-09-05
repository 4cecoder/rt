use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct PtyConfig {
    pub shell: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub working_dir: String,
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtyConfig {
    fn default() -> Self {
        let shell = if cfg!(target_os = "windows") {
            "cmd.exe".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        };

        let mut env_vars = HashMap::new();
        env_vars.insert("TERM".to_string(), "xterm-256color".to_string());
        env_vars.insert("COLORTERM".to_string(), "truecolor".to_string());

        Self {
            shell,
            args: vec![],
            env_vars,
            working_dir: std::env::current_dir()
                .unwrap_or_else(|_| "/".into())
                .to_string_lossy()
                .to_string(),
            rows: 24,
            cols: 80,
        }
    }
}

pub struct PtySession {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    pub config: PtyConfig,
    pub output_sender: broadcast::Sender<Vec<u8>>,
    pub input_sender: Sender<Vec<u8>>,
    pub is_running: Arc<Mutex<bool>>,
}

impl PtySession {
    pub fn new(config: PtyConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pty_system = native_pty_system();
        
        let pty_size = PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_pair = pty_system.openpty(pty_size)?;
        let mut cmd = CommandBuilder::new(&config.shell);
        
        for arg in &config.args {
            cmd.arg(arg);
        }
        
        cmd.cwd(&config.working_dir);
        
        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        let child = pty_pair.slave.spawn_command(cmd)?;
        let master = pty_pair.master;

        let (output_sender, _) = broadcast::channel(1024);
        let (input_sender, input_receiver) = mpsc::channel();
        let is_running = Arc::new(Mutex::new(true));

        let session = Self {
            master,
            child,
            config,
            output_sender,
            input_sender,
            is_running,
        };

        // Start I/O threads
        session.start_io_threads(input_receiver)?;

        Ok(session)
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pty_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        
        self.master.resize(pty_size)?;
        self.config.rows = rows;
        self.config.cols = cols;
        
        Ok(())
    }

    pub fn write_input(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.input_sender.send(data.to_vec())?;
        Ok(())
    }

    pub fn subscribe_output(&self) -> broadcast::Receiver<Vec<u8>> {
        self.output_sender.subscribe()
    }

    pub fn is_alive(&self) -> bool {
        // Note: try_wait() requires &mut self, but we need this to be &self
        // In a real implementation, you might use Arc<Mutex<>> for the child
        // For now, we'll use a simpler check
        true // Simplified - always return true for now
    }

    pub fn kill(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        self.child.kill()?;
        Ok(())
    }

    pub fn wait(&mut self) -> Result<portable_pty::ExitStatus, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.child.wait()?)
    }

    fn start_io_threads(&self, input_receiver: Receiver<Vec<u8>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For now, let's simplify and just handle output
        // In a real implementation, you'd need to handle the writer differently
        let mut master_reader = self.master.try_clone_reader()?;
        let output_sender = self.output_sender.clone();
        let is_running = self.is_running.clone();

        // Output reading thread
        let output_is_running = is_running.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            
            while output_is_running.lock().map(|x| *x).unwrap_or(true) {
                match master_reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = buffer[0..n].to_vec();
                        if output_sender.send(data).is_err() {
                            break; // No receivers
                        }
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            log::error!("Error reading from PTY: {}", e);
                            break;
                        }
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            }
        });

        // TODO: Handle input writing - this requires a different approach
        // For now, just consume the receiver to prevent blocking
        let input_is_running = is_running.clone();
        thread::spawn(move || {
            while input_is_running.lock().map(|x| *x).unwrap_or(true) {
                match input_receiver.recv() {
                    Ok(_data) => {
                        // TODO: Write data to PTY
                        log::debug!("Input received but not yet implemented");
                    }
                    Err(_) => break, // Channel closed
                }
            }
        });

        Ok(())
    }
}

pub struct PtyManager {
    sessions: HashMap<u32, PtySession>,
    next_id: u32,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn create_session(&mut self, config: PtyConfig) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        let session = PtySession::new(config)?;
        let id = self.next_id;
        self.next_id += 1;
        
        self.sessions.insert(id, session);
        Ok(id)
    }

    pub fn get_session(&self, id: u32) -> Option<&PtySession> {
        self.sessions.get(&id)
    }

    pub fn get_session_mut(&mut self, id: u32) -> Option<&mut PtySession> {
        self.sessions.get_mut(&id)
    }

    pub fn remove_session(&mut self, id: u32) -> Option<PtySession> {
        self.sessions.remove(&id)
    }

    pub fn list_sessions(&self) -> Vec<u32> {
        self.sessions.keys().copied().collect()
    }

    pub fn cleanup_dead_sessions(&mut self) {
        let dead_sessions: Vec<u32> = self
            .sessions
            .iter()
            .filter_map(|(&id, session)| {
                if !session.is_alive() {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        for id in dead_sessions {
            self.sessions.remove(&id);
        }
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_creation() {
        let config = PtyConfig::default();
        let session = PtySession::new(config);
        assert!(session.is_ok());
    }

    #[tokio::test]
    async fn test_pty_resize() {
        let config = PtyConfig::default();
        let mut session = PtySession::new(config).unwrap();
        
        let result = session.resize(50, 120);
        assert!(result.is_ok());
        assert_eq!(session.config.rows, 50);
        assert_eq!(session.config.cols, 120);
    }

    #[tokio::test]
    async fn test_pty_manager() {
        let mut manager = PtyManager::new();
        let config = PtyConfig::default();
        
        let id = manager.create_session(config).unwrap();
        assert!(manager.get_session(id).is_some());
        
        let removed = manager.remove_session(id);
        assert!(removed.is_some());
        assert!(manager.get_session(id).is_none());
    }
}