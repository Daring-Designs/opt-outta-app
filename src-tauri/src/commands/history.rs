use crate::history;
use crate::models::{BrokerSubmissionStatus, SubmissionRecord};

#[tauri::command]
pub fn get_submissions(app: tauri::AppHandle) -> Result<Vec<SubmissionRecord>, String> {
    let h = history::load(&app)?;
    Ok(h.records)
}

#[tauri::command]
pub fn get_latest_submissions(app: tauri::AppHandle) -> Result<Vec<SubmissionRecord>, String> {
    history::get_latest_per_broker(&app)
}

#[tauri::command]
pub fn get_relisting_alerts(app: tauri::AppHandle) -> Result<Vec<SubmissionRecord>, String> {
    history::get_due_for_recheck(&app)
}

#[tauri::command]
pub fn update_submission_status(
    app: tauri::AppHandle,
    id: String,
    status: BrokerSubmissionStatus,
) -> Result<(), String> {
    let mut h = history::load(&app)?;
    if let Some(record) = h.records.iter_mut().find(|r| r.id == id) {
        record.status = status;
        if record.status == BrokerSubmissionStatus::Confirmed {
            record.confirmed_at = Some(chrono::Utc::now());
        }
    } else {
        return Err("Submission record not found".to_string());
    }
    history::save(&app, &h)
}
