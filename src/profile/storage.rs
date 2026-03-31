use std::fs;
use std::path::PathBuf;

use crate::profile::model::VpnProfile;

fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("openvpn-gui");
    fs::create_dir_all(&dir).ok();
    dir
}

fn profiles_path() -> PathBuf {
    config_dir().join("profiles.json")
}

pub fn profiles_config_dir() -> PathBuf {
    let dir = config_dir().join("configs");
    fs::create_dir_all(&dir).ok();
    dir
}

pub fn load_profiles() -> Vec<VpnProfile> {
    let path = profiles_path();
    if !path.exists() {
        return Vec::new();
    }
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_profiles(profiles: &[VpnProfile]) {
    let path = profiles_path();
    if let Ok(data) = serde_json::to_string_pretty(profiles) {
        fs::write(path, data).ok();
    }
}
