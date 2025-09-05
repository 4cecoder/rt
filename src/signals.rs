use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;

#[cfg(unix)]
use nix::sys::signal::{self as nix_signal, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

#[derive(Debug, Clone)]
pub struct SignalHandler {
    pub shutdown_requested: Arc<AtomicBool>,
    pub resize_requested: Arc<AtomicBool>,
}

impl SignalHandler {
    pub fn new() -> Self {
        Self {
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            resize_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn setup_signal_handlers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Handle Ctrl+C (SIGINT)
        let shutdown_flag = self.shutdown_requested.clone();
        tokio::spawn(async move {
            if let Err(e) = signal::ctrl_c().await {
                log::error!("Failed to listen for ctrl_c signal: {}", e);
                return;
            }
            log::info!("Received Ctrl+C, initiating shutdown");
            shutdown_flag.store(true, Ordering::Relaxed);
        });

        #[cfg(unix)]
        {
            // Handle SIGTERM
            let shutdown_flag = self.shutdown_requested.clone();
            tokio::spawn(async move {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to create SIGTERM handler");
                
                if sigterm.recv().await.is_some() {
                    log::info!("Received SIGTERM, initiating shutdown");
                    shutdown_flag.store(true, Ordering::Relaxed);
                }
            });

            // Handle SIGWINCH (window resize)
            let resize_flag = self.resize_requested.clone();
            tokio::spawn(async move {
                let mut sigwinch = signal::unix::signal(signal::unix::SignalKind::window_change())
                    .expect("Failed to create SIGWINCH handler");
                
                while sigwinch.recv().await.is_some() {
                    log::debug!("Received SIGWINCH, window resized");
                    resize_flag.store(true, Ordering::Relaxed);
                }
            });

            // Handle SIGHUP (hangup)
            let shutdown_flag = self.shutdown_requested.clone();
            tokio::spawn(async move {
                let mut sighup = signal::unix::signal(signal::unix::SignalKind::hangup())
                    .expect("Failed to create SIGHUP handler");
                
                if sighup.recv().await.is_some() {
                    log::info!("Received SIGHUP, initiating shutdown");
                    shutdown_flag.store(true, Ordering::Relaxed);
                }
            });

            // Handle SIGQUIT
            let shutdown_flag = self.shutdown_requested.clone();
            tokio::spawn(async move {
                let mut sigquit = signal::unix::signal(signal::unix::SignalKind::quit())
                    .expect("Failed to create SIGQUIT handler");
                
                if sigquit.recv().await.is_some() {
                    log::info!("Received SIGQUIT, initiating shutdown");
                    shutdown_flag.store(true, Ordering::Relaxed);
                }
            });
        }

        Ok(())
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    pub fn is_resize_requested(&self) -> bool {
        self.resize_requested.load(Ordering::Relaxed)
    }

    pub fn clear_resize_request(&self) {
        self.resize_requested.store(false, Ordering::Relaxed);
    }

    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(unix)]
pub struct ProcessManager {
    process_groups: std::collections::HashMap<i32, Vec<Pid>>,
}

#[cfg(unix)]
impl ProcessManager {
    pub fn new() -> Self {
        Self {
            process_groups: std::collections::HashMap::new(),
        }
    }

    pub fn add_process_to_group(&mut self, group_id: i32, pid: Pid) {
        self.process_groups.entry(group_id).or_insert_with(Vec::new).push(pid);
    }

    pub fn remove_process_from_group(&mut self, group_id: i32, pid: Pid) {
        if let Some(pids) = self.process_groups.get_mut(&group_id) {
            pids.retain(|&p| p != pid);
            if pids.is_empty() {
                self.process_groups.remove(&group_id);
            }
        }
    }

    pub fn send_signal_to_process(&self, pid: Pid, sig: Signal) -> Result<(), nix::Error> {
        nix_signal::kill(pid, sig)
    }

    pub fn send_signal_to_group(&self, group_id: i32, sig: Signal) -> Result<(), nix::Error> {
        if let Some(pids) = self.process_groups.get(&group_id) {
            for &pid in pids {
                if let Err(e) = nix_signal::kill(pid, sig) {
                    log::warn!("Failed to send signal {:?} to process {}: {}", sig, pid, e);
                }
            }
        }
        Ok(())
    }

    pub fn terminate_group(&mut self, group_id: i32) -> Result<(), nix::Error> {
        // Send SIGTERM first
        self.send_signal_to_group(group_id, Signal::SIGTERM)?;
        
        // Give processes time to exit gracefully
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // Send SIGKILL to any remaining processes
        self.send_signal_to_group(group_id, Signal::SIGKILL)?;
        
        // Clean up the group
        self.process_groups.remove(&group_id);
        
        Ok(())
    }

    pub fn interrupt_group(&self, group_id: i32) -> Result<(), nix::Error> {
        self.send_signal_to_group(group_id, Signal::SIGINT)
    }

    pub fn suspend_group(&self, group_id: i32) -> Result<(), nix::Error> {
        self.send_signal_to_group(group_id, Signal::SIGTSTP)
    }

    pub fn resume_group(&self, group_id: i32) -> Result<(), nix::Error> {
        self.send_signal_to_group(group_id, Signal::SIGCONT)
    }

    pub fn list_groups(&self) -> Vec<i32> {
        self.process_groups.keys().copied().collect()
    }

    pub fn get_group_processes(&self, group_id: i32) -> Option<&Vec<Pid>> {
        self.process_groups.get(&group_id)
    }
}

#[cfg(windows)]
pub struct ProcessManager {
    process_handles: std::collections::HashMap<u32, winapi::um::winnt::HANDLE>,
}

#[cfg(windows)]
impl ProcessManager {
    pub fn new() -> Self {
        Self {
            process_handles: std::collections::HashMap::new(),
        }
    }

    pub fn add_process(&mut self, pid: u32, handle: winapi::um::winnt::HANDLE) {
        self.process_handles.insert(pid, handle);
    }

    pub fn remove_process(&mut self, pid: u32) {
        if let Some(handle) = self.process_handles.remove(&pid) {
            unsafe {
                winapi::um::handleapi::CloseHandle(handle);
            }
        }
    }

    pub fn terminate_process(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(&handle) = self.process_handles.get(&pid) {
            unsafe {
                if winapi::um::processthreadsapi::TerminateProcess(handle, 1) == 0 {
                    return Err("Failed to terminate process".into());
                }
            }
        }
        Ok(())
    }

    pub fn send_ctrl_c(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            if winapi::um::wincon::GenerateConsoleCtrlEvent(winapi::um::wincon::CTRL_C_EVENT, pid) == 0 {
                return Err("Failed to send Ctrl+C".into());
            }
        }
        Ok(())
    }

    pub fn send_ctrl_break(&self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            if winapi::um::wincon::GenerateConsoleCtrlEvent(winapi::um::wincon::CTRL_BREAK_EVENT, pid) == 0 {
                return Err("Failed to send Ctrl+Break".into());
            }
        }
        Ok(())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.as_str()
        } else {
            "Unknown panic occurred"
        };

        let location = if let Some(location) = panic_info.location() {
            format!(" at {}:{}", location.file(), location.line())
        } else {
            " at unknown location".to_string()
        };

        log::error!("Panic occurred: {}{}", message, location);
        eprintln!("Terminal emulator panic: {}{}", message, location);
        
        // Perform cleanup
        std::process::exit(1);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(!handler.is_shutdown_requested());
        assert!(!handler.is_resize_requested());
    }

    #[tokio::test]
    async fn test_signal_handler_flags() {
        let handler = SignalHandler::new();
        
        handler.request_shutdown();
        assert!(handler.is_shutdown_requested());
        
        handler.resize_requested.store(true, Ordering::Relaxed);
        assert!(handler.is_resize_requested());
        
        handler.clear_resize_request();
        assert!(!handler.is_resize_requested());
    }

    #[cfg(unix)]
    #[test]
    fn test_process_manager() {
        let mut manager = ProcessManager::new();
        let pid = Pid::from_raw(1234);
        
        manager.add_process_to_group(1, pid);
        assert_eq!(manager.get_group_processes(1), Some(&vec![pid]));
        
        manager.remove_process_from_group(1, pid);
        assert!(manager.get_group_processes(1).is_none());
    }
}