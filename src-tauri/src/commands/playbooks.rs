use crate::models::{ChangelogEntry, LocalPlaybook, Playbook, PlaybookReportEntry, PlaybookSubmission, PlaybookSubmitResponse, PlaybookSummary, RecordedAction, TrackedSubmission};
use crate::playbook_api;
use crate::playbook_verification;
use crate::recorder::RecorderState;
use crate::submission_tracker;

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
    let summaries = playbook_api::fetch_playbooks(&broker_id).await?;

    // Filter out playbooks that fail signature verification.
    // Build a temporary Playbook to reuse the verification function.
    let verified: Vec<PlaybookSummary> = summaries
        .into_iter()
        .filter(|s| {
            if s.steps.is_empty() || s.signature.is_none() {
                return false;
            }
            let pb = Playbook {
                id: s.id.clone(),
                broker_id: s.broker_id.clone(),
                broker_name: s.broker_name.clone(),
                title: s.title.clone(),
                version: s.version,
                status: "approved".to_string(),
                notes: s.notes.clone(),
                steps: s.steps.clone(),
                signature: s.signature.clone(),
                upvotes: s.upvotes,
                downvotes: s.downvotes,
                success_count: s.success_count,
                failure_count: s.failure_count,
                created_at: s.created_at.clone(),
            };
            playbook_verification::verify_playbook_signature(&pb).is_ok()
        })
        .collect();

    Ok(verified)
}

#[tauri::command]
pub async fn fetch_playbook_detail(id: String) -> Result<Playbook, String> {
    let playbook = playbook_api::fetch_playbook_detail(&id).await?;
    playbook_verification::verify_playbook_signature(&playbook)?;
    Ok(playbook)
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
        app_version: env!("CARGO_PKG_VERSION").to_string(),
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

// --- Submission tracker commands ---

#[tauri::command]
pub async fn track_submission(
    app: tauri::AppHandle,
    submission: TrackedSubmission,
) -> Result<(), String> {
    submission_tracker::track(&app, submission)
}

#[tauri::command]
pub async fn get_tracked_submissions(
    app: tauri::AppHandle,
) -> Result<Vec<TrackedSubmission>, String> {
    submission_tracker::get_all(&app)
}

#[tauri::command]
pub async fn refresh_submission_statuses(
    app: tauri::AppHandle,
) -> Result<Vec<TrackedSubmission>, String> {
    let subs = submission_tracker::get_all(&app)?;
    let terminal = ["approved", "rejected"];
    for sub in &subs {
        if !terminal.contains(&sub.status.as_str()) {
            if let Ok(new_status) = playbook_api::check_playbook_status(&sub.playbook_id).await {
                if new_status != sub.status {
                    let _ = submission_tracker::update_status(&app, &sub.playbook_id, &new_status);
                    // Clean up local draft when approved
                    if new_status == "approved" {
                        if let Some(local_id) = &sub.local_playbook_id {
                            let _ = crate::local_playbooks::delete(&app, local_id);
                        }
                    }
                }
            }
        }
    }
    submission_tracker::get_all(&app)
}

// --- Changelog command ---

#[tauri::command]
pub async fn fetch_changelog() -> Result<Vec<ChangelogEntry>, String> {
    playbook_api::fetch_changelog().await
}

// --- Playbook reports command ---

#[tauri::command]
pub async fn fetch_playbook_reports(id: String) -> Result<Vec<PlaybookReportEntry>, String> {
    playbook_api::fetch_playbook_reports(&id).await
}

// --- Broker suggestion command ---

#[tauri::command]
pub async fn suggest_broker(
    name: String,
    url: String,
    notes: String,
) -> Result<(), String> {
    playbook_api::suggest_broker(&name, &url, &notes).await
}
