use crate::models::{LocalPlaybook, LocalPlaybookStore};
use std::fs;
use std::path::PathBuf;

const FILENAME: &str = "local_playbooks.json";

pub fn store_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(FILENAME))
}

pub fn load(app: &tauri::AppHandle) -> Result<LocalPlaybookStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(LocalPlaybookStore::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save(app: &tauri::AppHandle, store: &LocalPlaybookStore) -> Result<(), String> {
    let path = store_path(app)?;
    let data = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn upsert(app: &tauri::AppHandle, playbook: LocalPlaybook) -> Result<(), String> {
    crate::playbook_validation::validate_steps(&playbook.steps)?;
    let mut store = load(app)?;
    if let Some(existing) = store.playbooks.iter_mut().find(|p| p.id == playbook.id) {
        *existing = playbook;
    } else {
        store.playbooks.push(playbook);
    }
    save(app, &store)
}

pub fn delete(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let mut store = load(app)?;
    store.playbooks.retain(|p| p.id != id);
    save(app, &store)
}

pub fn get_all(app: &tauri::AppHandle) -> Result<Vec<LocalPlaybook>, String> {
    let store = load(app)?;
    Ok(store.playbooks)
}
