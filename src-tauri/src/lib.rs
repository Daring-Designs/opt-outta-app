mod browser;
mod commands;
mod crypto;
mod engine;
mod history;
mod local_playbooks;
mod models;
mod playbook_validation;
mod playbook_verification;
mod playbook_api;
mod recorder;
mod secrets;
mod submission_tracker;

use commands::{brokers, history as history_cmd, optout, playbooks, profile};
use engine::EngineState;
use recorder::RecorderState;
use secrets::SecretsCache;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(EngineState(Arc::new(Mutex::new(None))))
        .manage(RecorderState::new())
        .setup(|app| {
            let secrets = SecretsCache::new();
            if let Err(e) = secrets.load() {
                eprintln!("Warning: Failed to load secrets from keychain: {}", e);
            }
            app.manage(secrets);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Profile
            profile::save_profile,
            profile::get_profile,
            profile::delete_profile,
            // Brokers
            brokers::get_brokers,
            // Opt-out
            optout::check_chrome_installed,
            optout::start_opt_out_run,
            optout::continue_opt_out,
            optout::cancel_opt_out,
            optout::get_run_status,
            // History
            history_cmd::get_submissions,
            history_cmd::get_latest_submissions,
            history_cmd::get_relisting_alerts,
            history_cmd::update_submission_status,
            // Playbooks
            playbooks::start_recording,
            playbooks::stop_recording,
            playbooks::get_recorded_actions,
            playbooks::mark_captcha_step,
            playbooks::mark_user_prompt_step,
            playbooks::fetch_playbooks,
            playbooks::fetch_playbook_detail,
            playbooks::submit_playbook,
            playbooks::vote_on_playbook,
            playbooks::report_playbook_outcome,
            // Local playbooks
            playbooks::save_local_playbook,
            playbooks::get_local_playbooks,
            playbooks::delete_local_playbook,
            // Submission tracker
            playbooks::track_submission,
            playbooks::get_tracked_submissions,
            playbooks::refresh_submission_statuses,
            // Broker suggestions
            playbooks::suggest_broker,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
