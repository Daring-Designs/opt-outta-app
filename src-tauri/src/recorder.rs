use crate::browser;
use crate::models::RecordedAction;
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// JavaScript injected into pages during recording mode.
/// Records user actions (clicks, form interactions, navigations) but NEVER captures field values or PII.
const RECORDER_JS: &str = r#"
(() => {
    if (window.__optOuttaRecorder) return;

    window.__optOuttaRecorder = {
        actions: [],
        lastClickTime: 0,
        lastClickSelector: ''
    };

    function cssSelector(el) {
        if (el.id) return '#' + CSS.escape(el.id);
        if (el.name && (el.tagName === 'INPUT' || el.tagName === 'SELECT' || el.tagName === 'TEXTAREA')) {
            return el.tagName.toLowerCase() + '[name="' + CSS.escape(el.name) + '"]';
        }
        let path = [];
        while (el && el.nodeType === 1) {
            let selector = el.tagName.toLowerCase();
            if (el.id) { path.unshift('#' + CSS.escape(el.id)); break; }
            let sib = el, nth = 1;
            while (sib = sib.previousElementSibling) { if (sib.tagName === el.tagName) nth++; }
            if (nth > 1) selector += ':nth-of-type(' + nth + ')';
            path.unshift(selector);
            el = el.parentElement;
        }
        return path.join(' > ');
    }

    function getLabel(field) {
        if (field.id) {
            const label = document.querySelector('label[for="' + CSS.escape(field.id) + '"]');
            if (label) return label.textContent.trim();
        }
        const parent = field.closest('label');
        if (parent) return parent.textContent.trim();
        const prev = field.previousElementSibling;
        if (prev && prev.tagName === 'LABEL') return prev.textContent.trim();
        return field.getAttribute('aria-label') || null;
    }

    function inferProfileKey(field) {
        const hints = [
            field.name || '',
            field.id || '',
            field.placeholder || '',
            field.getAttribute('autocomplete') || '',
            (getLabel(field) || '').toLowerCase()
        ].join(' ').toLowerCase();

        if (/first.?name|given.?name|fname/i.test(hints)) return 'firstName';
        if (/last.?name|family.?name|lname|surname/i.test(hints)) return 'lastName';
        if (/email/i.test(hints)) return 'email';
        if (/phone|tel/i.test(hints)) return 'phone';
        if (/street|address(?!.*city|.*state|.*zip)/i.test(hints)) return 'address';
        if (/city/i.test(hints)) return 'city';
        if (/state|province/i.test(hints)) return 'state';
        if (/zip|postal/i.test(hints)) return 'zip';
        if (/dob|birth|birthday/i.test(hints)) return 'dob';
        return null;
    }

    // Track form field interactions (blur = user finished typing)
    document.addEventListener('blur', (e) => {
        const el = e.target;
        if (!el || !['INPUT', 'SELECT', 'TEXTAREA'].includes(el.tagName)) return;
        if (el.type === 'hidden') return;

        const selector = cssSelector(el);
        const profileKey = inferProfileKey(el);
        const label = getLabel(el);

        if (el.tagName === 'SELECT') {
            window.__optOuttaRecorder.actions.push({
                action: 'select',
                selector: selector,
                profile_key: profileKey,
                value: null,
                url: null,
                element_text: null,
                label: label,
                timestamp: Date.now()
            });
        } else if (el.type === 'checkbox' || el.type === 'radio') {
            window.__optOuttaRecorder.actions.push({
                action: 'check',
                selector: selector,
                profile_key: null,
                value: el.checked ? 'true' : 'false',
                url: null,
                element_text: null,
                label: label,
                timestamp: Date.now()
            });
        } else {
            window.__optOuttaRecorder.actions.push({
                action: 'fill',
                selector: selector,
                profile_key: profileKey,
                value: null,
                url: null,
                element_text: null,
                label: label,
                timestamp: Date.now()
            });
        }
    }, true);

    // Track clicks on buttons, links, and other interactive elements
    document.addEventListener('click', (e) => {
        const el = e.target.closest('button, a, input[type="submit"], [role="button"], .btn');
        if (!el) return;
        // Skip form fields — those are handled by blur
        if (['INPUT', 'SELECT', 'TEXTAREA'].includes(el.tagName) && el.type !== 'submit') return;

        const selector = cssSelector(el);
        const now = Date.now();

        // Deduplicate rapid clicks on same selector
        if (selector === window.__optOuttaRecorder.lastClickSelector && now - window.__optOuttaRecorder.lastClickTime < 500) {
            return;
        }
        window.__optOuttaRecorder.lastClickSelector = selector;
        window.__optOuttaRecorder.lastClickTime = now;

        window.__optOuttaRecorder.actions.push({
            action: 'click',
            selector: selector,
            profile_key: null,
            value: null,
            url: null,
            element_text: (el.textContent || el.value || '').trim().substring(0, 100),
            label: null,
            timestamp: now
        });
    }, true);
})()
"#;

/// State for an active recording session.
pub struct ActiveRecording {
    #[allow(dead_code)]
    pub broker_id: String,
    #[allow(dead_code)]
    pub broker_name: String,
    browser: Browser,
    _handler_task: JoinHandle<()>,
    page: Arc<Page>,
    poll_task: JoinHandle<()>,
    actions: Arc<Mutex<Vec<RecordedAction>>>,
}

/// Managed state wrapper for the recorder.
pub struct RecorderState(pub Mutex<Option<ActiveRecording>>);

impl RecorderState {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

/// Start a recording session: launch Chrome, navigate to opt-out URL, inject recorder JS.
pub async fn start_recording(
    state: &RecorderState,
    broker_id: String,
    broker_name: String,
    opt_out_url: String,
) -> Result<(), String> {
    let mut guard = state.0.lock().await;
    if guard.is_some() {
        return Err("A recording session is already active.".to_string());
    }

    let (browser, mut handler) = browser::launch().await?;
    let handler_task = tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(_) = handler.next().await {}
    });

    let page = browser
        .new_page(&opt_out_url)
        .await
        .map_err(|e| format!("Failed to open page: {}", e))?;

    sleep(Duration::from_secs(2)).await;

    // Inject recorder JS
    page.evaluate(RECORDER_JS)
        .await
        .map_err(|e| format!("Failed to inject recorder: {}", e))?;

    let page = Arc::new(page);
    let actions: Arc<Mutex<Vec<RecordedAction>>> = Arc::new(Mutex::new(Vec::new()));

    // Start URL polling loop to detect navigation and re-inject JS
    let poll_page = Arc::clone(&page);
    let poll_actions = Arc::clone(&actions);
    let poll_task = tokio::spawn(async move {
        let mut last_url = String::new();
        loop {
            sleep(Duration::from_secs(2)).await;

            let url_result = poll_page
                .evaluate("window.location.href")
                .await;

            let current_url = match url_result {
                Ok(val) => val.into_value::<String>().unwrap_or_default(),
                Err(_) => break, // Page/browser closed
            };

            if !current_url.is_empty() && current_url != last_url {
                if !last_url.is_empty() {
                    // Record navigation
                    poll_actions.lock().await.push(RecordedAction {
                        action: "navigate".to_string(),
                        selector: None,
                        profile_key: None,
                        value: Some(current_url.clone()),
                        url: Some(current_url.clone()),
                        element_text: None,
                        label: None,
                        timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    });
                }
                last_url = current_url;

                // Re-inject recorder JS on new page
                let _ = poll_page.evaluate(RECORDER_JS).await;
            }

            // Collect any actions the JS has recorded
            let extract_result = poll_page
                .evaluate("(() => { const a = (window.__optOuttaRecorder || {}).actions || []; window.__optOuttaRecorder.actions = []; return a; })()")
                .await;

            if let Ok(val) = extract_result {
                if let Ok(new_actions) = val.into_value::<Vec<RecordedAction>>() {
                    if !new_actions.is_empty() {
                        poll_actions.lock().await.extend(new_actions);
                    }
                }
            }
        }
    });

    *guard = Some(ActiveRecording {
        broker_id,
        broker_name,
        browser,
        _handler_task: handler_task,
        page,
        poll_task,
        actions,
    });

    Ok(())
}

/// Mark a CAPTCHA step at the current point in the recording.
pub async fn mark_captcha(state: &RecorderState) -> Result<(), String> {
    let guard = state.0.lock().await;
    let recording = guard.as_ref().ok_or("No active recording session.")?;

    recording.actions.lock().await.push(RecordedAction {
        action: "captcha".to_string(),
        selector: None,
        profile_key: None,
        value: None,
        url: None,
        element_text: None,
        label: Some("User solved CAPTCHA".to_string()),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    });

    Ok(())
}

/// Mark a user prompt step at the current point in the recording.
pub async fn mark_user_prompt(state: &RecorderState) -> Result<(), String> {
    let guard = state.0.lock().await;
    let recording = guard.as_ref().ok_or("No active recording session.")?;

    recording.actions.lock().await.push(RecordedAction {
        action: "user_prompt".to_string(),
        selector: None,
        profile_key: None,
        value: None,
        url: None,
        element_text: None,
        label: Some("Manual step".to_string()),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    });

    Ok(())
}

/// Get a snapshot of the current recorded actions without stopping.
pub async fn get_current_actions(state: &RecorderState) -> Result<Vec<RecordedAction>, String> {
    let guard = state.0.lock().await;
    let recording = guard.as_ref().ok_or("No active recording session.")?;
    let actions = recording.actions.lock().await.clone();
    Ok(actions)
}

/// Stop the recording session and return all recorded actions.
pub async fn stop_recording(state: &RecorderState) -> Result<Vec<RecordedAction>, String> {
    let mut guard = state.0.lock().await;
    let mut recording = guard.take().ok_or("No active recording session.")?;

    // Stop the polling loop
    recording.poll_task.abort();

    // Extract any final actions from the page
    let final_result = recording
        .page
        .evaluate("(() => { const a = (window.__optOuttaRecorder || {}).actions || []; window.__optOuttaRecorder.actions = []; return a; })()")
        .await;

    let mut all_actions = recording.actions.lock().await.clone();

    if let Ok(val) = final_result {
        if let Ok(final_actions) = val.into_value::<Vec<RecordedAction>>() {
            all_actions.extend(final_actions);
        }
    }

    // Close browser
    let _ = recording.browser.close().await;

    // Sort by timestamp
    all_actions.sort_by_key(|a| a.timestamp);

    Ok(all_actions)
}
