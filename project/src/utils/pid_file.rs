use std::fs;
use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process;

/// Manages a PID file to prevent multiple daemon instances
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Creates a PID file or exits the process on error.
    /// Automatically stops any existing daemon.
    pub fn create_or_exit(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        match Self::create(path) {
            Ok(pid_file) => pid_file,
            Err(e) => {
                eprintln!("Error creating PID file: {}", e);
                process::exit(1);
            }
        }
    }

    /// Creates a PID file. If daemon is already running, kills it first.
    pub fn create(path: PathBuf) -> io::Result<Self> {
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if Self::is_process_running(pid) {
                        println!("Found existing daemon (PID: {}). Stopping it...", pid);
                        Self::kill_process(pid)?;
                        // Wait a bit for the process to die and ports to be freed
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        println!("Previous daemon stopped successfully.");
                    }
                }
            }
            fs::remove_file(&path)?;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let pid = process::id();
        fs::write(&path, pid.to_string())?;
        Ok(PidFile { path })
    }

    /// Checks if a process with given PID is running
    #[cfg(unix)]
    fn is_process_running(pid: u32) -> bool {
        use std::process::Command;
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        use std::process::Command;
        Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }

    /// Kills a process by PID
    #[cfg(unix)]
    fn kill_process(pid: u32) -> io::Result<()> {
        use std::process::Command;
        let status = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                ErrorKind::Other,
                format!("Failed to kill process {}", pid),
            ))
        }
    }

    #[cfg(windows)]
    fn kill_process(pid: u32) -> io::Result<()> {
        use std::process::Command;
        let status = Command::new("taskkill")
            .args(&["/F", "/PID", &pid.to_string()])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                ErrorKind::Other,
                format!("Failed to kill process {}", pid),
            ))
        }
    }
}

impl Drop for PidFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
