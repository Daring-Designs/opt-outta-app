use crate::models::BrokerRegistry;
use tauri::Manager;

#[tauri::command]
pub fn get_brokers(app: tauri::AppHandle) -> Result<BrokerRegistry, String> {
    let resource_path = app
        .path()
        .resolve("brokers.json", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;

    let data = std::fs::read_to_string(&resource_path)
        .map_err(|e| format!("Failed to read broker registry at {:?}: {}", resource_path, e))?;

    let registry: BrokerRegistry =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse broker registry: {}", e))?;

    Ok(registry)
}
