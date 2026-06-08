use anyhow::{Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;

fn runtime_dir() -> PathBuf {
    let dir = PathBuf::from("/tmp/openvpn-gui");
    fs::create_dir_all(&dir).ok();
    dir
}

fn pid_path(profile_name: &str) -> PathBuf {
    runtime_dir().join(format!("{}.pid", profile_name))
}

fn log_path(profile_name: &str) -> PathBuf {
    runtime_dir().join(format!("{}.log", profile_name))
}

fn auth_path(profile_name: &str) -> PathBuf {
    runtime_dir().join(format!("{}.auth", profile_name))
}

fn mgmt_port(profile_name: &str) -> u16 {
    let hash: u32 = profile_name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    (hash % 10000 + 20000) as u16
}

fn mgmt_port_path(profile_name: &str) -> PathBuf {
    runtime_dir().join(format!("{}.mgmt", profile_name))
}

pub fn connect(profile_name: &str, config_path: &str, username: &str, password: &str) -> Result<()> {
    if is_connected(profile_name) {
        anyhow::bail!("Profile '{}' is already connected", profile_name);
    }

    let auth_file = auth_path(profile_name);
    fs::write(&auth_file, format!("{}\n{}\n", username, password))
        .context("Failed to write auth file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&auth_file, fs::Permissions::from_mode(0o600)).ok();
    }

    let pid_file = pid_path(profile_name);
    let log_file = log_path(profile_name);
    let port = mgmt_port(profile_name);

    fs::write(mgmt_port_path(profile_name), port.to_string()).ok();

    let output = Command::new("pkexec")
        .arg("openvpn")
        .arg("--config")
        .arg(config_path)
        .arg("--auth-user-pass")
        .arg(&auth_file)
        .arg("--daemon")
        .arg(format!("openvpn-gui-{}", profile_name))
        .arg("--writepid")
        .arg(&pid_file)
        .arg("--log")
        .arg(&log_file)
        .arg("--management")
        .arg("127.0.0.1")
        .arg(port.to_string())
        .arg("--setenv")
        .arg("PATH")
        .arg("/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
        .output()
        .context("Failed to execute openvpn")?;

    fs::remove_file(&auth_file).ok();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("openvpn failed: {}", stderr.trim());
    }

    std::thread::sleep(std::time::Duration::from_millis(500));

    if !is_connected(profile_name) {
        let log_content = fs::read_to_string(&log_file).unwrap_or_default();
        let last_lines: String = log_content.lines().rev().take(5).collect::<Vec<_>>().join("\n");
        anyhow::bail!("openvpn exited immediately. Log:\n{}", last_lines);
    }

    log::info!("VPN connected: {}", profile_name);
    Ok(())
}

pub fn disconnect(profile_name: &str) -> Result<()> {
    let port_file = mgmt_port_path(profile_name);
    let port_str = fs::read_to_string(&port_file)
        .unwrap_or_else(|_| mgmt_port(profile_name).to_string());
    let port: u16 = port_str.trim().parse().unwrap_or_else(|_| mgmt_port(profile_name));

    match TcpStream::connect(format!("127.0.0.1:{}", port)) {
        Ok(mut stream) => {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = stream.write_all(b"signal SIGTERM\n");
            let _ = stream.flush();
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        Err(e) => {
            log::warn!("Could not connect to management socket: {}", e);
        }
    }

    fs::remove_file(pid_path(profile_name)).ok();
    fs::remove_file(&port_file).ok();

    std::thread::sleep(std::time::Duration::from_millis(500));

    if is_connected(profile_name) {
        anyhow::bail!("Failed to disconnect, process still running");
    }

    log::info!("VPN disconnected: {}", profile_name);
    Ok(())
}

pub fn is_connected(profile_name: &str) -> bool {
    let pid_file = pid_path(profile_name);
    if !pid_file.exists() {
        return false;
    }
    let pid_str = match fs::read_to_string(&pid_file) {
        Ok(s) => s.trim().to_string(),
        Err(_) => return false,
    };
    let proc_path = format!("/proc/{}", pid_str);
    std::path::Path::new(&proc_path).exists()
}

#[allow(dead_code)]
pub fn read_stats(profile_name: &str) -> Option<(u64, u64)> {
    let port_file = mgmt_port_path(profile_name);
    let port_str = fs::read_to_string(&port_file).ok()?;
    let port: u16 = port_str.trim().parse().ok()?;

    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).ok()?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(300))).ok();

    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf);

    stream.write_all(b"load-stats\n").ok()?;
    stream.flush().ok()?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    let n = stream.read(&mut buf).ok()?;
    let response = String::from_utf8_lossy(&buf[..n]);

    for line in response.lines() {
        if line.starts_with("SUCCESS:") {
            let bytes_in = line.split(',')
                .find(|s| s.contains("bytesin="))
                .and_then(|s| s.split('=').last())
                .and_then(|v| v.trim().parse::<u64>().ok())?;
            let bytes_out = line.split(',')
                .find(|s| s.contains("bytesout="))
                .and_then(|s| s.split('=').last())
                .and_then(|v| v.trim().parse::<u64>().ok())?;
            return Some((bytes_in, bytes_out));
        }
    }
    None
}

pub fn disconnect_all() {
    let dir = runtime_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pid") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                if is_connected(name) {
                    log::info!("Disconnecting VPN on quit: {}", name);
                    let _ = disconnect(name);
                }
            }
        }
    }
}

pub fn read_stats_and_info(profile_name: &str) -> Option<(u64, u64, String, String)> {
    let port_file = mgmt_port_path(profile_name);
    let port_str = fs::read_to_string(&port_file).ok()?;
    let port: u16 = port_str.trim().parse().ok()?;

    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).ok()?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok();

    let mut buf = [0u8; 8192];
    let _ = stream.read(&mut buf);

    stream.write_all(b"load-stats\nstate\n").ok()?;
    stream.flush().ok()?;
    std::thread::sleep(std::time::Duration::from_millis(100));

    let n = stream.read(&mut buf).ok()?;
    let response = String::from_utf8_lossy(&buf[..n]);

    let mut bytes_in: Option<u64> = None;
    let mut bytes_out: Option<u64> = None;
    let mut vpn_ip = String::new();
    let mut server_ip = String::new();

    for line in response.lines() {
        if line.starts_with("SUCCESS:") {
            if let Some(bi) = line.split(',')
                .find(|s| s.contains("bytesin="))
                .and_then(|s| s.split('=').last())
                .and_then(|v| v.trim().parse::<u64>().ok())
            {
                bytes_in = Some(bi);
            }
            if let Some(bo) = line.split(',')
                .find(|s| s.contains("bytesout="))
                .and_then(|s| s.split('=').last())
                .and_then(|v| v.trim().parse::<u64>().ok())
            {
                bytes_out = Some(bo);
            }
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 5 && parts.get(1) == Some(&"CONNECTED") {
            let ip = parts[3].to_string();
            let srv = parts[4].to_string();
            if !ip.is_empty() {
                vpn_ip = ip;
                server_ip = srv;
            }
        }
    }

    Some((bytes_in?, bytes_out?, vpn_ip, server_ip))
}

#[allow(dead_code)]
pub fn read_connection_info(profile_name: &str) -> Option<(String, String)> {
    let port_file = mgmt_port_path(profile_name);
    let port_str = fs::read_to_string(&port_file).ok()?;
    let port: u16 = port_str.trim().parse().ok()?;

    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).ok()?;
    stream.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok();

    let mut buf = [0u8; 4096];
    let _ = stream.read(&mut buf);

    stream.write_all(b"state\n").ok()?;
    stream.flush().ok()?;
    std::thread::sleep(std::time::Duration::from_millis(80));

    let n = stream.read(&mut buf).ok()?;
    let response = String::from_utf8_lossy(&buf[..n]);

    for line in response.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 5 && parts.get(1) == Some(&"CONNECTED") {
            let vpn_ip = parts[3].to_string();
            let server_ip = parts[4].to_string();
            if !vpn_ip.is_empty() && !server_ip.is_empty() {
                return Some((vpn_ip, server_ip));
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn read_log(profile_name: &str) -> String {
    let log_file = log_path(profile_name);
    fs::read_to_string(&log_file).unwrap_or_default()
}
