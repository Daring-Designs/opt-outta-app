use crate::crypto;
use crate::models::Profile;
use crate::secrets::SecretsCache;
use std::fs;
use tauri::Manager;

const PROFILE_FILENAME: &str = "profile.enc";

fn profile_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(PROFILE_FILENAME))
}

#[tauri::command]
pub fn save_profile(app: tauri::AppHandle, profile: Profile) -> Result<(), String> {
    let secrets = app.state::<SecretsCache>();
    let key = secrets.get_encryption_key()?;
    let json = serde_json::to_string(&profile).map_err(|e| e.to_string())?;
    let encrypted = crypto::encrypt(json.as_bytes(), &key).map_err(|e| e.to_string())?;
    let path = profile_path(&app)?;
    fs::write(&path, encrypted).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_profile(app: tauri::AppHandle) -> Result<Option<Profile>, String> {
    let path = profile_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let encrypted = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let secrets = app.state::<SecretsCache>();
    let key = secrets.get_encryption_key()?;
    let decrypted = crypto::decrypt(&encrypted, &key).map_err(|e| e.to_string())?;
    let profile: Profile = serde_json::from_slice(&decrypted).map_err(|e| e.to_string())?;
    Ok(Some(profile))
}

#[tauri::command]
pub fn delete_profile(app: tauri::AppHandle) -> Result<(), String> {
    let path = profile_path(&app)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
