use crate::model::AppConfig;
use std::fs;
use std::path::PathBuf;

fn config_path() -> Result<PathBuf, String> {
    let executable =
        std::env::current_exe().map_err(|e| format!("resolve executable path: {e}"))?;
    let directory = executable
        .parent()
        .ok_or("could not resolve executable directory")?;
    Ok(directory.join("config.json"))
}

pub fn load() -> Result<AppConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse config: {e}"))
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path()?;
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| format!("create config dir: {e}"))?;

    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("serialize config: {e}"))?;

    // Atomic write: temp file then rename.
    let tmp = dir.join("config.json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("write config: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("replace config: {e}"))?;
    Ok(())
}
