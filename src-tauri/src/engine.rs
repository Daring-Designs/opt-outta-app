use crate::browser;
use crate::history;
use crate::local_playbooks;
use crate::models::*;
use crate::playbook_api;
use crate::playbook_validation;
use crate::playbook_verification;
use chrono::{Duration, Utc};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Convert a PlaybookStep (from API) to a FormAction (for browser::execute_action).
fn playbook_step_to_form_action(step: &PlaybookStep) -> Option<FormAction> {
    match step.action.as_str() {
        "navigate" => Some(FormAction::Navigate {
            url: step.value.clone().unwrap_or_default(),
        }),
        "fill" => {
            if step.profile_key.is_some() {
                Some(FormAction::Fill {
                    selector: step.selector.clone().unwrap_or_default(),
                    profile_key: step.profile_key.clone(),
                    value: None,
                    transform: None,
                })
            } else {
                Some(FormAction::ManualFill {
                    selector: step.selector.clone().unwrap_or_default(),
                    message: step.description.clone(),
                })
            }
        }
        "select" => {
            // Select can use profile_key or a static value
            let value = step.value.clone().unwrap_or_else(|| {
                step.profile_key.clone().unwrap_or_default()
            });
            Some(FormAction::Select {
                selector: step.selector.clone().unwrap_or_default(),
                value,
            })
        }
        "check" => Some(FormAction::Check {
            selector: step.selector.clone().unwrap_or_default(),
            checked: step.value.as_deref() != Some("false"),
        }),
        "click" => Some(FormAction::Click {
            selector: step.selector.clone().unwrap_or_default(),
        }),
        "wait" => Some(FormAction::Wait {
            milliseconds: step.wait_after_ms as u64,
        }),
        "wait_for" => Some(FormAction::WaitFor {
            selector: step.selector.clone().unwrap_or_default(),
            timeout_ms: Some(10000),
        }),
        "scroll_to" => Some(FormAction::ScrollTo {
            selector: step.selector.clone().unwrap_or_default(),
        }),
        "find_and_click" => Some(FormAction::FindAndClick {
            selector: step.selector.clone().unwrap_or_default(),
            profile_key: step.profile_key.clone().unwrap_or_default(),
        }),
        "captcha" => Some(FormAction::Captcha {
            message: Some(step.description.clone()),
        }),
        "user_prompt" => Some(FormAction::UserPrompt {
            message: step.description.clone(),
        }),
        _ => None,
    }
}

pub struct EngineState(pub Arc<Mutex<Option<OptOutEngine>>>);

pub struct OptOutEngine {
    #[allow(dead_code)]
    pub run_id: String,
    pub status: RunStatus,
    cancel_tx: Option<oneshot::Sender<()>>,
    user_action_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl OptOutEngine {
    pub fn new(run_id: String) -> (Self, oneshot::Receiver<()>) {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let engine = Self {
            run_id,
            status: RunStatus::Running,
            cancel_tx: Some(cancel_tx),
            user_action_tx: Arc::new(Mutex::new(None)),
        };
        (engine, cancel_rx)
    }

    pub fn cancel(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }
        self.status = RunStatus::Failed;
    }

    pub async fn signal_user_action(&self) {
        let mut guard = self.user_action_tx.lock().await;
        if let Some(tx) = guard.take() {
            let _ = tx.send(());
        }
    }

    pub fn user_action_channel(&self) -> Arc<Mutex<Option<oneshot::Sender<()>>>> {
        self.user_action_tx.clone()
    }
}

pub async fn run_opt_outs(
    app: tauri::AppHandle,
    run_id: String,
    brokers: Vec<Broker>,
    profile: Profile,
    playbook_selections: std::collections::HashMap<String, String>,
    user_action_channel: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    use tauri::Emitter;

    let total = brokers.len();

    // Launch browser
    let emit_progress = |broker: &Broker, step: &str, completed: usize, status: RunStatus, action: Option<UserActionRequired>, error: Option<String>| {
        let progress = OptOutProgress {
            run_id: run_id.clone(),
            broker_id: broker.id.clone(),
            broker_name: broker.name.clone(),
            status,
            current_step: step.to_string(),
            brokers_completed: completed,
            brokers_total: total,
            action_required: action,
            error,
        };
        let _ = app.emit("opt-out-progress", &progress);
    };

    let (browser_instance, mut handler) = match browser::launch().await {
        Ok(b) => b,
        Err(e) => {
            if let Some(broker) = brokers.first() {
                emit_progress(broker, "Failed to launch Chrome", 0, RunStatus::Failed, None, Some(e));
            }
            let _ = app.emit("opt-out-complete", serde_json::json!({
                "run_id": run_id, "total": total, "succeeded": 0, "failed": total
            }));
            return;
        }
    };

    // Spawn handler in background — Handler implements Stream<Item = Result<()>>
    let handler_handle = tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (idx, broker) in brokers.iter().enumerate() {
        // Check for cancellation
        if cancel_rx.try_recv().is_ok() {
            emit_progress(broker, "Cancelled", idx, RunStatus::Failed, None, Some("Run cancelled by user".to_string()));
            break;
        }

        emit_progress(broker, "Navigating to opt-out page...", idx, RunStatus::Running, None, None);

        // Open new page
        let page = match browser_instance.new_page(&broker.opt_out_url).await {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("Failed to open page: {}", e);
                emit_progress(broker, &error_msg, idx, RunStatus::Running, None, Some(error_msg.clone()));
                save_failed_record(&app, broker, &run_id, &error_msg);
                failed += 1;
                continue;
            }
        };

        // Wait for page load
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut broker_success = false;

        // --- Playbook path ---
        let playbook = if let Some(selection) = playbook_selections.get(&broker.id) {
            if selection.starts_with("local:") {
                // Local playbook selected — load from local storage
                let local_id = &selection[6..];
                match local_playbooks::get_all(&app) {
                    Ok(locals) => locals
                        .into_iter()
                        .find(|lp| lp.id == local_id)
                        .map(|lp| Playbook {
                            id: lp.id,
                            broker_id: lp.broker_id,
                            broker_name: lp.broker_name,
                            version: 0,
                            status: "local".to_string(),
                            notes: lp.notes,
                            steps: lp.steps,
                            signature: None,
                            upvotes: 0,
                            downvotes: 0,
                            success_count: 0,
                            failure_count: 0,
                            created_at: lp.created_at,
                        }),
                    Err(_) => None,
                }
            } else if selection == "best" {
                match playbook_api::fetch_best_playbook(&broker.id).await {
                    Ok(pb) => pb,
                    Err(_) => None,
                }
            } else {
                // Specific community playbook ID selected
                match playbook_api::fetch_playbook_detail(selection).await {
                    Ok(pb) => Some(pb),
                    Err(_) => None,
                }
            }
        } else {
            None
        };

        // Require a playbook for every broker
        let pb = match playbook {
            Some(pb) => pb,
            None => {
                let error_msg = "No playbook available for this broker".to_string();
                emit_progress(broker, &error_msg, idx, RunStatus::Running, None, Some(error_msg.clone()));
                save_failed_record(&app, broker, &run_id, &error_msg);
                failed += 1;
                let _ = page.close().await;
                continue;
            }
        };

        // Verify signature on community playbooks before executing
        if pb.status != "local" {
            if let Err(e) = playbook_verification::verify_playbook_signature(&pb) {
                let error_msg = format!("Playbook rejected: {}", e);
                emit_progress(broker, &error_msg, idx, RunStatus::Running, None, Some(error_msg.clone()));
                save_failed_record(&app, broker, &run_id, &error_msg);
                failed += 1;
                let _ = page.close().await;
                continue;
            }
        }

        // Validate playbook steps before executing
        if let Err(validation_err) = playbook_validation::validate_steps(&pb.steps) {
            let error_msg = format!("Playbook rejected: {}", validation_err);
            emit_progress(broker, &error_msg, idx, RunStatus::Running, None, Some(error_msg.clone()));
            save_failed_record(&app, broker, &run_id, &error_msg);
            failed += 1;
            let _ = page.close().await;
            continue;
        }

        let is_local = pb.status == "local";
        let label = if is_local { "local playbook" } else { &format!("community playbook v{}", pb.version) };
        emit_progress(broker, &format!("Using {}...", label), idx, RunStatus::Running, None, None);

        let mut playbook_failed = false;
        let mut failure_step: Option<u32> = None;
        let mut failure_error: Option<String> = None;

        for step in &pb.steps {
            if cancel_rx.try_recv().is_ok() {
                break;
            }

            let form_action = match playbook_step_to_form_action(step) {
                Some(a) => a,
                None => continue,
            };

            emit_progress(broker, &step.description, idx, RunStatus::Running, None, None);

            match &form_action {
                FormAction::Captcha { message } => {
                    let msg = message.as_deref().unwrap_or("Please solve the CAPTCHA.");
                    emit_progress(
                        broker, msg, idx, RunStatus::WaitingForUser,
                        Some(UserActionRequired::SolveCaptcha {
                            captcha_type: None,
                            message: msg.to_string(),
                        }),
                        None,
                    );
                    let (tx, rx) = oneshot::channel();
                    {
                        let mut guard = user_action_channel.lock().await;
                        *guard = Some(tx);
                    }
                    let _ = rx.await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                FormAction::UserPrompt { message } => {
                    emit_progress(
                        broker, message, idx, RunStatus::WaitingForUser,
                        Some(UserActionRequired::UserPrompt {
                            message: message.clone(),
                        }),
                        None,
                    );
                    let (tx, rx) = oneshot::channel();
                    {
                        let mut guard = user_action_channel.lock().await;
                        *guard = Some(tx);
                    }
                    let _ = rx.await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                FormAction::ManualFill { selector, message } => {
                    // Scroll to and highlight the field in the browser
                    if let Err(e) = browser::highlight_element(&page, selector).await {
                        if !step.optional {
                            playbook_failed = true;
                            failure_step = Some(step.position);
                            failure_error = Some(e.clone());
                            emit_progress(broker, &format!("Failed to highlight field: {}", e), idx, RunStatus::Running, None, None);
                            break;
                        }
                        continue;
                    }
                    emit_progress(
                        broker, message, idx, RunStatus::WaitingForUser,
                        Some(UserActionRequired::UserPrompt {
                            message: format!("Please fill out this field in the browser: {}", message),
                        }),
                        None,
                    );
                    let (tx, rx) = oneshot::channel();
                    {
                        let mut guard = user_action_channel.lock().await;
                        *guard = Some(tx);
                    }
                    let _ = rx.await;
                    // Remove the highlight after user confirms
                    let _ = browser::remove_highlight(&page, selector).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
                other => {
                    if let Err(e) = browser::execute_action(&page, other, &profile).await {
                        if step.optional {
                            continue;
                        }
                        playbook_failed = true;
                        failure_step = Some(step.position);
                        failure_error = Some(e.clone());
                        emit_progress(broker, &format!("Playbook step failed: {}", e), idx, RunStatus::Running, None, None);
                        break;
                    }
                }
            }

            // Wait after step
            if step.wait_after_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(step.wait_after_ms as u64)).await;
            }
        }

        // Report outcome to API for community playbooks (fire and forget)
        if !is_local {
            let playbook_id = pb.id.clone();
            let outcome_str = if playbook_failed { "failure" } else { "success" }.to_string();
            let report = PlaybookReport {
                device_id: playbook_api::get_device_id(),
                outcome: outcome_str,
                failure_step,
                error_message: failure_error.clone(),
                app_version: "0.1.0".to_string(),
            };
            tokio::spawn(async move {
                let _ = playbook_api::report_outcome(&playbook_id, &report).await;
            });
        }

        if !playbook_failed {
            broker_success = true;
        }

        // Save record
        if broker_success {
            save_success_record(&app, broker, &run_id);
            succeeded += 1;
            emit_progress(broker, "Opt-out submitted", idx + 1, RunStatus::Running, None, None);
        } else {
            let err = failure_error.unwrap_or_else(|| "Playbook execution failed".to_string());
            save_failed_record(&app, broker, &run_id, &err);
            failed += 1;
        }

        // Close the page
        let _ = page.close().await;
    }

    // Emit completion
    let _ = app.emit("opt-out-complete", serde_json::json!({
        "run_id": run_id,
        "total": total,
        "succeeded": succeeded,
        "failed": failed
    }));

    // Clean up browser
    drop(browser_instance);
    handler_handle.abort();
}

fn save_success_record(app: &tauri::AppHandle, broker: &Broker, run_id: &str) {
    let status = if broker.requires_verification.is_some() {
        BrokerSubmissionStatus::PendingVerification
    } else {
        BrokerSubmissionStatus::Submitted
    };
    let next_check = broker.relist_days.map(|days| Utc::now() + Duration::days(days as i64));
    let record = SubmissionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        broker_id: broker.id.clone(),
        status,
        submitted_at: Utc::now(),
        confirmed_at: None,
        next_check_date: next_check,
        error_message: None,
        run_id: run_id.to_string(),
    };
    let _ = history::upsert_record(app, record);
}

fn save_failed_record(app: &tauri::AppHandle, broker: &Broker, run_id: &str, error: &str) {
    let record = SubmissionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        broker_id: broker.id.clone(),
        status: BrokerSubmissionStatus::Failed,
        submitted_at: Utc::now(),
        confirmed_at: None,
        next_check_date: None,
        error_message: Some(error.to_string()),
        run_id: run_id.to_string(),
    };
    let _ = history::upsert_record(app, record);
}
