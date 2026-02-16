use crate::models::BrokerRegistry;
use crate::playbook_api;
use crate::registry_cache;
use tauri::Manager;

#[tauri::command]
pub fn get_brokers(app: tauri::AppHandle) -> Result<BrokerRegistry, String> {
    // Load bundled registry
    let resource_path = app
        .path()
        .resolve("brokers.json", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let data = std::fs::read_to_string(&resource_path)
        .map_err(|e| format!("Failed to read broker registry at {:?}: {}", resource_path, e))?;

    let bundled: BrokerRegistry =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse broker registry: {}", e))?;

    // Prefer cached registry if its version is newer
    if let Ok(Some(cached)) = registry_cache::load(&app) {
        if cached.version > bundled.version {
            return Ok(cached);
        }
    }

    Ok(bundled)
}

#[tauri::command]
pub async fn sync_registry(app: tauri::AppHandle) -> Result<bool, String> {
    // Get current version (cached or bundled)
    let current = get_brokers(app.clone())?;
    let current_version = current.version;

    // Check remote version
    let remote_version = playbook_api::fetch_registry_version().await?;

    if remote_version <= current_version {
        return Ok(false);
    }

    // Download and cache the new registry
    let registry = playbook_api::fetch_registry().await?;
    registry_cache::save(&app, &registry)?;

    Ok(true)
}
