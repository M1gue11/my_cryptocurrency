use crate::db::db::init_db;
use crate::globals::CONFIG;
use crate::model::get_node;
use crate::network::server::run_server;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug)]
pub enum DaemonCommand {
    Start,
    Stop,
    Status,
    Restart,
}

pub struct DaemonInfo {
    pub pid: u32,
    pub port: u16,
    pub start_time: String,
}

fn get_pid_file_path() -> PathBuf {
    PathBuf::from(&CONFIG.persisted_chain_path).join("daemon.pid")
}

pub fn write_pid_file() -> Result<(), String> {
    let pid = process::id();
    let port = 7777u16;
    let start_time = chrono::Local::now().to_rfc3339();

    let content = format!("{}\n{}\n{}", pid, port, start_time);
    let pid_file = get_pid_file_path();

    fs::create_dir_all(pid_file.parent().unwrap())
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    fs::write(&pid_file, content).map_err(|e| format!("Failed to write PID file: {}", e))?;

    Ok(())
}

pub fn read_pid_file() -> Result<DaemonInfo, String> {
    let pid_file = get_pid_file_path();
    let content =
        fs::read_to_string(&pid_file).map_err(|e| format!("Failed to read PID file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 3 {
        return Err("Invalid PID file format".to_string());
    }

    let pid = lines[0]
        .parse::<u32>()
        .map_err(|_| "Invalid PID format".to_string())?;
    let port = lines[1]
        .parse::<u16>()
        .map_err(|_| "Invalid port format".to_string())?;
    let start_time = lines[2].to_string();

    Ok(DaemonInfo {
        pid,
        port,
        start_time,
    })
}

pub fn remove_pid_file() -> Result<(), String> {
    let pid_file = get_pid_file_path();
    if pid_file.exists() {
        fs::remove_file(&pid_file).map_err(|e| format!("Failed to remove PID file: {}", e))?;
    }
    Ok(())
}

pub fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(&pid.to_string())
        } else {
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::fs;
        fs::metadata(format!("/proc/{}", pid)).is_ok()
    }
}

pub fn is_daemon_running() -> Result<bool, String> {
    match read_pid_file() {
        Ok(info) => Ok(is_process_running(info.pid)),
        Err(_) => Ok(false),
    }
}

pub async fn start_daemon() -> Result<(), String> {
    // Check if daemon already running
    if is_daemon_running()? {
        return Err("Daemon is already running".to_string());
    }

    // Write PID file
    write_pid_file()?;

    println!("Starting daemon...");

    // Initialize database
    init_db();

    // Start P2P server
    let port = CONFIG.p2p_port;
    let peers = CONFIG.peers.clone();
    let p2p_handle = tokio::spawn(async move {
        run_server(port, peers).await;
    });

    // Start RPC server
    let rpc_handle = tokio::spawn(async move {
        use crate::daemon::rpc_server::run_rpc_server;
        if let Err(e) = run_rpc_server().await {
            eprintln!("RPC server error: {}", e);
        }
    });

    // Wait for shutdown signal or task completion
    tokio::select! {
        _ = wait_for_shutdown_signal() => {},
        _ = p2p_handle => {},
        _ = rpc_handle => {},
    }

    // Cleanup
    cleanup_daemon().await?;

    Ok(())
}

pub async fn stop_daemon() -> Result<(), String> {
    let info = read_pid_file()?;

    if !is_process_running(info.pid) {
        remove_pid_file()?;
        return Err("Daemon is not running (stale PID file removed)".to_string());
    }

    println!("Stopping daemon (PID: {})...", info.pid);

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("taskkill")
            .arg("/PID")
            .arg(info.pid.to_string())
            .arg("/F")
            .output()
            .map_err(|e| format!("Failed to stop daemon: {}", e))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        Command::new("kill")
            .arg(info.pid.to_string())
            .output()
            .map_err(|e| format!("Failed to stop daemon: {}", e))?;
    }

    // Wait a bit for process to terminate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Remove PID file
    remove_pid_file()?;

    println!("✓ Daemon stopped");
    Ok(())
}

pub async fn show_daemon_status() -> Result<(), String> {
    match read_pid_file() {
        Ok(info) => {
            if is_process_running(info.pid) {
                println!("✓ Daemon is running (PID: {})", info.pid);
                println!("  RPC endpoint: http://127.0.0.1:{}", info.port);
                println!("  Started at: {}", info.start_time);
            } else {
                println!("✗ Daemon is not running (stale PID file)");
                remove_pid_file()?;
            }
        }
        Err(_) => {
            println!("✗ Daemon is not running");
        }
    }
    Ok(())
}

pub async fn restart_daemon() -> Result<(), String> {
    println!("Restarting daemon...");

    if let Ok(_) = read_pid_file() {
        stop_daemon().await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    start_daemon().await
}

async fn wait_for_shutdown_signal() {
    #[cfg(target_os = "windows")]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("\nReceived Ctrl+C signal");
    }

    #[cfg(not(target_os = "windows"))]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => println!("\nReceived SIGTERM"),
            _ = sigint.recv() => println!("\nReceived SIGINT"),
        }
    }
}

async fn cleanup_daemon() -> Result<(), String> {
    println!("Cleaning up daemon...");

    // Save node state
    let node = get_node().await;
    node.save_node();

    // Remove PID file
    remove_pid_file()?;

    println!("✓ Daemon stopped cleanly");
    Ok(())
}

pub async fn handle_daemon_command(cmd: DaemonCommand) -> Result<(), String> {
    match cmd {
        DaemonCommand::Start => start_daemon().await,
        DaemonCommand::Stop => stop_daemon().await,
        DaemonCommand::Status => show_daemon_status().await,
        DaemonCommand::Restart => restart_daemon().await,
    }
}
