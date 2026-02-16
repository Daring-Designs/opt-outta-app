use crate::models::{SubmissionHistory, SubmissionRecord};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

const HISTORY_FILENAME: &str = "submissions.json";

pub fn history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(HISTORY_FILENAME))
}

pub fn load(app: &tauri::AppHandle) -> Result<SubmissionHistory, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(SubmissionHistory::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn save(app: &tauri::AppHandle, history: &SubmissionHistory) -> Result<(), String> {
    let path = history_path(app)?;
    let data = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| e.to_string())
}

pub fn upsert_record(app: &tauri::AppHandle, record: SubmissionRecord) -> Result<(), String> {
    let mut history = load(app)?;
    if let Some(existing) = history.records.iter_mut().find(|r| r.id == record.id) {
        *existing = record;
    } else {
        history.records.push(record);
    }
    save(app, &history)
}

#[allow(dead_code)]
pub fn get_by_broker(app: &tauri::AppHandle, broker_id: &str) -> Result<Vec<SubmissionRecord>, String> {
    let history = load(app)?;
    Ok(history
        .records
        .into_iter()
        .filter(|r| r.broker_id == broker_id)
        .collect())
}

pub fn get_latest_per_broker(app: &tauri::AppHandle) -> Result<Vec<SubmissionRecord>, String> {
    let history = load(app)?;
    let mut latest: std::collections::HashMap<String, SubmissionRecord> = std::collections::HashMap::new();
    for record in history.records {
        let entry = latest.entry(record.broker_id.clone()).or_insert_with(|| record.clone());
        if record.submitted_at > entry.submitted_at {
            *entry = record;
        }
    }
    Ok(latest.into_values().collect())
}

pub fn get_due_for_recheck(app: &tauri::AppHandle) -> Result<Vec<SubmissionRecord>, String> {
    let now = Utc::now();
    let latest = get_latest_per_broker(app)?;
    Ok(latest
        .into_iter()
        .filter(|r| {
            r.next_check_date
                .map(|d| d <= now)
                .unwrap_or(false)
        })
        .collect())
}
