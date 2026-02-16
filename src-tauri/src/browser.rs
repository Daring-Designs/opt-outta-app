use crate::models::{FormAction, PageStructure, Profile};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::handler::Handler;
use chromiumoxide::page::Page;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

/// Find the Chrome binary on this platform.
pub fn find_chrome_binary() -> Option<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
    } else {
        vec![
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
        ]
    };

    candidates
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

/// Shut down any stale Chrome process left over from a previous opt-out run.
/// Chrome's SingletonLock is a symlink whose target is "{hostname}-{pid}".
fn cleanup_previous_chrome(data_dir: &Path) {
    let lock_path = data_dir.join("SingletonLock");

    if let Ok(target) = std::fs::read_link(&lock_path) {
        let target_str = target.to_string_lossy();
        if let Some(pid_str) = target_str.rsplit('-').next() {
            if pid_str.parse::<u32>().is_ok() {
                // Kill the leftover automation Chrome (this is our process, not the user's browser)
                let _ = std::process::Command::new("kill").arg(pid_str).output();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    }

    // Remove stale lock and socket files
    let _ = std::fs::remove_file(&lock_path);
    let _ = std::fs::remove_file(data_dir.join("SingletonSocket"));
}

/// Launch a visible (headful) Chrome instance.
pub async fn launch() -> Result<(Browser, Handler), String> {
    let chrome_path = find_chrome_binary()
        .ok_or_else(|| "Chrome not found. Please install Google Chrome.".to_string())?;

    // Use a dedicated data dir so we don't conflict with the user's Chrome
    let data_dir = std::env::temp_dir().join("opt-outta-chrome");
    cleanup_previous_chrome(&data_dir);

    let config = BrowserConfig::builder()
        .with_head()
        .chrome_executable(chrome_path)
        .user_data_dir(&data_dir)
        .arg("--disable-blink-features=AutomationControlled")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-timer-throttling")
        .viewport(None)
        .build()
        .map_err(|e| format!("Failed to build browser config: {}", e))?;

    Browser::launch(config)
        .await
        .map_err(|e| format!("Failed to launch Chrome: {}", e))
}

/// Navigate to a URL and wait for load.
pub async fn navigate(page: &Page, url: &str) -> Result<(), String> {
    page.goto(url)
        .await
        .map_err(|e| format!("Navigation failed: {}", e))?;
    sleep(Duration::from_secs(2)).await;
    Ok(())
}

/// JavaScript that extracts page structure without reading any field values.
#[allow(dead_code)]
const EXTRACT_JS: &str = r#"
(() => {
    function cssSelector(el) {
        if (el.id) return '#' + CSS.escape(el.id);
        if (el.name && el.tagName === 'INPUT') return `input[name="${CSS.escape(el.name)}"]`;
        if (el.name && el.tagName === 'SELECT') return `select[name="${CSS.escape(el.name)}"]`;
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
            const label = document.querySelector(`label[for="${CSS.escape(field.id)}"]`);
            if (label) return label.textContent.trim();
        }
        const parent = field.closest('label');
        if (parent) return parent.textContent.trim().replace(field.value || '', '').trim();
        const prev = field.previousElementSibling;
        if (prev && prev.tagName === 'LABEL') return prev.textContent.trim();
        return field.getAttribute('aria-label') || null;
    }

    function isVisible(el) {
        const style = window.getComputedStyle(el);
        return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetParent !== null;
    }

    const forms = Array.from(document.querySelectorAll('form')).map(form => {
        const fields = Array.from(form.querySelectorAll('input, select, textarea')).map(f => ({
            selector: cssSelector(f),
            tag: f.tagName.toLowerCase(),
            type: f.type || null,
            label: getLabel(f),
            placeholder: f.placeholder || null,
            name: f.name || null,
            id: f.id || null,
            required: f.required || false,
            options: f.tagName === 'SELECT' ? Array.from(f.options).map(o => o.textContent.trim()) : null,
            visible: isVisible(f)
        }));
        return {
            selector: cssSelector(form),
            action: form.action || null,
            method: form.method || null,
            fields
        };
    });

    const buttons = Array.from(document.querySelectorAll('button, input[type="submit"], a[role="button"]')).map(b => ({
        selector: cssSelector(b),
        text: (b.textContent || b.value || '').trim(),
        type: b.type || null,
        visible: isVisible(b)
    }));

    const textBlocks = Array.from(document.querySelectorAll('h1, h2, h3, p.important, .notice, .alert'))
        .slice(0, 10)
        .map(el => el.textContent.trim())
        .filter(t => t.length > 0 && t.length < 500);

    // Check for CAPTCHA presence, but also check if it's already solved
    const captchaWidgetPresent = !!(
        document.querySelector('[class*="captcha" i]') ||
        document.querySelector('[id*="captcha" i]') ||
        document.querySelector('iframe[src*="recaptcha"]') ||
        document.querySelector('iframe[src*="hcaptcha"]') ||
        document.querySelector('.g-recaptcha') ||
        document.querySelector('.h-captcha')
    );
    let hasCaptcha = captchaWidgetPresent;
    if (captchaWidgetPresent) {
        // reCAPTCHA: solved when g-recaptcha-response textarea has a non-empty value
        const recaptchaResponse = document.querySelector('textarea[name="g-recaptcha-response"]');
        if (recaptchaResponse && recaptchaResponse.value && recaptchaResponse.value.length > 0) {
            hasCaptcha = false;
        }
        // hCaptcha: solved when h-captcha-response textarea has a non-empty value
        const hcaptchaResponse = document.querySelector('textarea[name="h-captcha-response"]');
        if (hcaptchaResponse && hcaptchaResponse.value && hcaptchaResponse.value.length > 0) {
            hasCaptcha = false;
        }
    }

    return {
        url: window.location.href,
        title: document.title,
        forms,
        buttons,
        text_blocks: textBlocks,
        has_captcha: hasCaptcha
    };
})()
"#;

/// Extract page structure (no PII — only labels, types, selectors).
#[allow(dead_code)]
pub async fn extract_page_structure(page: &Page) -> Result<PageStructure, String> {
    let result = page
        .evaluate(EXTRACT_JS)
        .await
        .map_err(|e| format!("Failed to extract page structure: {}", e))?;

    let value = result.into_value::<serde_json::Value>()
        .map_err(|e| format!("Failed to convert JS result: {}", e))?;

    serde_json::from_value(value).map_err(|e| format!("Failed to parse page structure: {}", e))
}

/// Resolve a profile key to the actual PII value (local only — never sent to API).
fn resolve_profile_key(profile: &Profile, key: &str, transform: Option<&str>) -> Option<String> {
    if let Some(transform_str) = transform {
        if transform_str.starts_with("combine:") {
            let keys: Vec<&str> = transform_str
                .strip_prefix("combine:")
                .unwrap()
                .split('+')
                .collect();
            let parts: Vec<String> = keys
                .iter()
                .filter_map(|k| resolve_profile_key(profile, k, None))
                .collect();
            return if parts.is_empty() { None } else { Some(parts.join(" ")) };
        }
    }

    match key {
        "firstName" => Some(profile.first_name.clone()),
        "lastName" => Some(profile.last_name.clone()),
        "email" => Some(profile.email.clone()),
        "phone" => Some(profile.phone.clone()),
        "address" => Some(profile.address.clone()),
        "city" => Some(profile.city.clone()),
        "state" => Some(profile.state.clone()),
        "zip" => Some(profile.zip.clone()),
        "dob" => Some(profile.dob.clone()),
        _ => None,
    }
}

/// Execute a single form action on the page.
pub async fn execute_action(page: &Page, action: &FormAction, profile: &Profile) -> Result<(), String> {
    // Human-like delay between actions
    let delay = Duration::from_millis(500 + (rand::random::<u64>() % 1000));
    sleep(delay).await;

    match action {
        FormAction::Fill { selector, profile_key, value, transform } => {
            let value = if let Some(ref pk) = profile_key {
                resolve_profile_key(profile, pk, transform.as_deref())
                    .ok_or_else(|| format!("Unknown profile key: {}", pk))?
            } else {
                value.clone().unwrap_or_default()
            };
            let sel_json = serde_json::to_string(selector).unwrap();
            let js = format!(
                r#"(() => {{
                    const sel = {sel};
                    const el = document.querySelector(sel);
                    if (!el) throw new Error('Element not found: ' + sel);
                    el.focus();
                    el.value = {val};
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                }})()"#,
                sel = sel_json,
                val = serde_json::to_string(&value).unwrap(),
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("Fill failed for {}: {}", selector, e))?;
        }
        FormAction::Select { selector, value } => {
            let js = format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) throw new Error('Element not found');
                    el.value = {val};
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                }})()"#,
                sel = serde_json::to_string(selector).unwrap(),
                val = serde_json::to_string(value).unwrap(),
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("Select failed for {}: {}", selector, e))?;
        }
        FormAction::Check { selector, checked } => {
            let js = format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) throw new Error('Element not found');
                    el.checked = {checked};
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                }})()"#,
                sel = serde_json::to_string(selector).unwrap(),
                checked = checked,
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("Check failed for {}: {}", selector, e))?;
        }
        FormAction::Click { selector } => {
            let js = format!(
                r#"(() => {{
                    const el = document.querySelector({sel});
                    if (!el) throw new Error('Element not found');
                    el.click();
                }})()"#,
                sel = serde_json::to_string(selector).unwrap(),
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("Click failed for {}: {}", selector, e))?;
        }
        FormAction::Wait { milliseconds } => {
            let capped = (*milliseconds).min(30_000);
            sleep(Duration::from_millis(capped)).await;
        }
        FormAction::Navigate { url } => {
            let lower = url.to_lowercase();
            if !lower.starts_with("http://") && !lower.starts_with("https://") {
                return Err(format!("Navigation blocked: only http/https URLs allowed, got: {}", url));
            }
            navigate(page, url).await?;
        }
        FormAction::WaitFor { selector, timeout_ms } => {
            let timeout = timeout_ms.unwrap_or(10000).min(30_000);
            let sel_json = serde_json::to_string(selector).unwrap();
            let js = format!(
                r#"(() => {{
                    return new Promise((resolve, reject) => {{
                        const sel = {sel};
                        const start = Date.now();
                        const poll = () => {{
                            if (document.querySelector(sel)) return resolve(true);
                            if (Date.now() - start > {timeout}) return reject(new Error('Timeout waiting for: ' + sel));
                            setTimeout(poll, 500);
                        }};
                        poll();
                    }});
                }})()"#,
                sel = sel_json,
                timeout = timeout,
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("WaitFor failed for {}: {}", selector, e))?;
        }
        FormAction::ScrollTo { selector } => {
            let sel_json = serde_json::to_string(selector).unwrap();
            let js = format!(
                r#"(() => {{
                    const sel = {sel};
                    const el = document.querySelector(sel);
                    if (!el) throw new Error('Element not found: ' + sel);
                    el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
                }})()"#,
                sel = sel_json,
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("ScrollTo failed for {}: {}", selector, e))?;
        }
        FormAction::FindAndClick { selector, profile_key } => {
            let value = resolve_profile_key(profile, profile_key, None)
                .ok_or_else(|| format!("Unknown profile key: {}", profile_key))?;
            let sel_json = serde_json::to_string(selector).unwrap();
            let js = format!(
                r#"(() => {{
                    const sel = {sel};
                    const containers = Array.from(document.querySelectorAll(sel));
                    const target = {val}.toLowerCase();
                    const found = containers.find(el => el.textContent.toLowerCase().includes(target));
                    if (!found) throw new Error('No element matching profile value found in: ' + sel);
                    found.click();
                }})()"#,
                sel = sel_json,
                val = serde_json::to_string(&value).unwrap(),
            );
            page.evaluate(js)
                .await
                .map_err(|e| format!("FindAndClick failed for {}: {}", selector, e))?;
        }
        // Captcha, UserPrompt, ManualFill, ManualSelect, Done, Error are handled by the engine, not here
        FormAction::Captcha { .. } | FormAction::UserPrompt { .. } | FormAction::ManualFill { .. } | FormAction::ManualSelect { .. } | FormAction::Done { .. } | FormAction::Error { .. } => {}
    }

    Ok(())
}

/// Scrolls to an element and adds a pulsing highlight border
pub async fn highlight_element(page: &Page, selector: &str) -> Result<(), String> {
    let sel_json = serde_json::to_string(selector).unwrap();
    let js = format!(
        r#"(() => {{
            const sel = {sel};
            const el = document.querySelector(sel);
            if (!el) throw new Error('Element not found: ' + sel);
            el.scrollIntoView({{ behavior: 'smooth', block: 'center' }});
            el.style.outline = '3px solid #3b82f6';
            el.style.outlineOffset = '2px';
            el.style.transition = 'outline-color 0.5s ease-in-out';
            el.dataset.optOuttaHighlight = 'true';
            let on = true;
            const iv = setInterval(() => {{
                on = !on;
                el.style.outlineColor = on ? '#3b82f6' : '#93c5fd';
            }}, 500);
            window.__optOuttaHighlightInterval = iv;
        }})()"#,
        sel = sel_json,
    );
    page.evaluate(js)
        .await
        .map_err(|e| format!("Failed to highlight element {}: {}", selector, e))?;
    Ok(())
}

/// Removes the highlight border from a previously highlighted element
pub async fn remove_highlight(page: &Page, selector: &str) -> Result<(), String> {
    let sel_json = serde_json::to_string(selector).unwrap();
    let js = format!(
        r#"(() => {{
            if (window.__optOuttaHighlightInterval) {{
                clearInterval(window.__optOuttaHighlightInterval);
                window.__optOuttaHighlightInterval = null;
            }}
            const sel = {sel};
            const el = document.querySelector(sel);
            if (el) {{
                el.style.outline = '';
                el.style.outlineOffset = '';
                el.style.transition = '';
                delete el.dataset.optOuttaHighlight;
            }}
        }})()"#,
        sel = sel_json,
    );
    page.evaluate(js)
        .await
        .map_err(|e| format!("Failed to remove highlight {}: {}", selector, e))?;
    Ok(())
}
