use crate::models::{LocalPlaybook, PlaybookSubmission, PlaybookSubmitResponse, PlaybookSummary, Playbook, RecordedAction};
use crate::playbook_api;
use crate::recorder::RecorderState;

// --- Recording commands ---

#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, RecorderState>,
    broker_id: String,
    broker_name: String,
    opt_out_url: String,
) -> Result<(), String> {
    crate::recorder::start_recording(&state, broker_id, broker_name, opt_out_url).await
}

#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, RecorderState>,
) -> Result<Vec<RecordedAction>, String> {
    crate::recorder::stop_recording(&state).await
}

#[tauri::command]
pub async fn get_recorded_actions(
    state: tauri::State<'_, RecorderState>,
) -> Result<Vec<RecordedAction>, String> {
    crate::recorder::get_current_actions(&state).await
}

#[tauri::command]
pub async fn mark_captcha_step(
    state: tauri::State<'_, RecorderState>,
) -> Result<(), String> {
    crate::recorder::mark_captcha(&state).await
}

#[tauri::command]
pub async fn mark_user_prompt_step(
    state: tauri::State<'_, RecorderState>,
) -> Result<(), String> {
    crate::recorder::mark_user_prompt(&state).await
}

// --- Playbook API commands ---

#[tauri::command]
pub async fn fetch_playbooks(broker_id: String) -> Result<Vec<PlaybookSummary>, String> {
    playbook_api::fetch_playbooks(&broker_id).await
}

#[tauri::command]
pub async fn fetch_playbook_detail(id: String) -> Result<Playbook, String> {
    playbook_api::fetch_playbook_detail(&id).await
}

#[tauri::command]
pub async fn submit_playbook(submission: PlaybookSubmission) -> Result<PlaybookSubmitResponse, String> {
    crate::playbook_validation::validate_steps(&submission.steps)?;
    playbook_api::submit_playbook(&submission).await
}

#[tauri::command]
pub async fn vote_on_playbook(id: String, vote: String) -> Result<(), String> {
    playbook_api::vote_playbook(&id, &vote).await
}

#[tauri::command]
pub async fn report_playbook_outcome(
    id: String,
    outcome: String,
    failure_step: Option<u32>,
    error_message: Option<String>,
) -> Result<(), String> {
    let report = crate::models::PlaybookReport {
        device_id: playbook_api::get_device_id(),
        outcome,
        failure_step,
        error_message,
        app_version: "0.1.0".to_string(),
    };
    playbook_api::report_outcome(&id, &report).await
}

// --- Local playbook commands ---

#[tauri::command]
pub async fn save_local_playbook(
    app: tauri::AppHandle,
    playbook: LocalPlaybook,
) -> Result<(), String> {
    crate::local_playbooks::upsert(&app, playbook)
}

#[tauri::command]
pub async fn get_local_playbooks(
    app: tauri::AppHandle,
) -> Result<Vec<LocalPlaybook>, String> {
    crate::local_playbooks::get_all(&app)
}

#[tauri::command]
pub async fn delete_local_playbook(
    app: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    crate::local_playbooks::delete(&app, &id)
}
