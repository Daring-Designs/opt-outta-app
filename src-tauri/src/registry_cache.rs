use crate::models::BrokerRegistry;
use std::fs;
use std::path::PathBuf;

const FILENAME: &str = "registry_cache.json";

fn cache_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(FILENAME))
}

pub fn load(app: &tauri::AppHandle) -> Result<Option<BrokerRegistry>, String> {
    let path = cache_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let registry: BrokerRegistry =
        serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(Some(registry))
}

pub fn save(app: &tauri::AppHandle, registry: &BrokerRegistry) -> Result<(), String> {
    let path = cache_path(app)?;
    let data = serde_json::to_string_pretty(registry).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}
