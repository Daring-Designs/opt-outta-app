use crate::models::{SubmissionTrackerStore, TrackedSubmission};
use std::fs;
use std::path::PathBuf;

const FILENAME: &str = "submission_tracker.json";

fn store_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(FILENAME))
}

fn load(app: &tauri::AppHandle) -> Result<SubmissionTrackerStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(SubmissionTrackerStore::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn save(app: &tauri::AppHandle, store: &SubmissionTrackerStore) -> Result<(), String> {
    let path = store_path(app)?;
    let data = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn track(app: &tauri::AppHandle, submission: TrackedSubmission) -> Result<(), String> {
    let mut store = load(app)?;
    // Replace if same playbook_id already tracked
    store.submissions.retain(|s| s.playbook_id != submission.playbook_id);
    store.submissions.push(submission);
    save(app, &store)
}

pub fn get_all(app: &tauri::AppHandle) -> Result<Vec<TrackedSubmission>, String> {
    let store = load(app)?;
    Ok(store.submissions)
}

pub fn update_status(app: &tauri::AppHandle, playbook_id: &str, status: &str) -> Result<(), String> {
    let mut store = load(app)?;
    if let Some(sub) = store.submissions.iter_mut().find(|s| s.playbook_id == playbook_id) {
        sub.status = status.to_string();
    }
    save(app, &store)
}
