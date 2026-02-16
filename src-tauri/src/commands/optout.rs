use crate::browser;
use crate::commands::profile;
use crate::engine::{self, EngineState, OptOutEngine};
use crate::models::{Broker, RunStatus};
use tauri::State;

#[tauri::command]
pub fn check_chrome_installed() -> bool {
    browser::find_chrome_binary().is_some()
}

#[tauri::command]
pub async fn start_opt_out_run(
    app: tauri::AppHandle,
    state: State<'_, EngineState>,
    broker_ids: Vec<String>,
    playbook_selections: Option<std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    // Check if already running
    {
        let guard = state.0.lock().await;
        if let Some(ref engine) = *guard {
            if engine.status == RunStatus::Running || engine.status == RunStatus::WaitingForUser {
                return Err("An opt-out run is already in progress".to_string());
            }
        }
    }

    // Load profile
    let prof = profile::get_profile(app.clone())?
        .ok_or_else(|| "No profile saved. Please set up your profile first.".to_string())?;

    // Load brokers
    let registry = crate::commands::brokers::get_brokers(app.clone())?;
    let selected_brokers: Vec<Broker> = registry
        .brokers
        .into_iter()
        .filter(|b| broker_ids.contains(&b.id))
        .collect();

    if selected_brokers.is_empty() {
        return Err("No valid brokers selected".to_string());
    }

    // Require a playbook selection for every broker
    let pb_selections = playbook_selections.unwrap_or_default();
    let missing: Vec<&str> = selected_brokers
        .iter()
        .filter(|b| !pb_selections.contains_key(&b.id))
        .map(|b| b.name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "No playbook selected for: {}. Select a playbook for each broker before running.",
            missing.join(", ")
        ));
    }

    let run_id = uuid::Uuid::new_v4().to_string();
    let (engine, cancel_rx) = OptOutEngine::new(run_id.clone());
    let user_action_channel = engine.user_action_channel();

    // Store engine in state
    {
        let mut guard = state.0.lock().await;
        *guard = Some(engine);
    }

    // Spawn the run in background
    let run_id_clone = run_id.clone();
    let state_clone = state.0.clone();
    tokio::spawn(async move {
        engine::run_opt_outs(
            app,
            run_id_clone,
            selected_brokers,
            prof,
            pb_selections,
            user_action_channel,
            cancel_rx,
        )
        .await;

        // Mark engine as completed
        let mut guard = state_clone.lock().await;
        if let Some(ref mut eng) = *guard {
            if eng.status == RunStatus::Running {
                eng.status = RunStatus::Completed;
            }
        }
    });

    Ok(run_id)
}

#[tauri::command]
pub async fn continue_opt_out(state: State<'_, EngineState>) -> Result<(), String> {
    let guard = state.0.lock().await;
    if let Some(ref engine) = *guard {
        engine.signal_user_action().await;
        Ok(())
    } else {
        Err("No active opt-out run".to_string())
    }
}

#[tauri::command]
pub async fn cancel_opt_out(state: State<'_, EngineState>) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    if let Some(ref mut engine) = *guard {
        engine.cancel();
        Ok(())
    } else {
        Err("No active opt-out run".to_string())
    }
}

#[tauri::command]
pub async fn get_run_status(state: State<'_, EngineState>) -> Result<RunStatus, String> {
    let guard = state.0.lock().await;
    match &*guard {
        Some(engine) => Ok(engine.status.clone()),
        None => Ok(RunStatus::Idle),
    }
}
