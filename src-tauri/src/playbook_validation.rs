use crate::models::PlaybookStep;

const MAX_STEPS: usize = 100;
const MAX_SELECTOR_LEN: usize = 500;
const MAX_VALUE_LEN: usize = 2000;
const MAX_DESCRIPTION_LEN: usize = 500;
const MAX_INSTRUCTIONS_LEN: usize = 2000;
const MAX_WAIT_MS: u32 = 30_000;

const ALLOWED_ACTIONS: &[&str] = &[
    "navigate",
    "fill",
    "select",
    "check",
    "click",
    "wait",
    "wait_for",
    "scroll_to",
    "find_and_click",
    "captcha",
    "user_prompt",
    "done",
];

const ALLOWED_PROFILE_KEYS: &[&str] = &[
    "firstName",
    "lastName",
    "email",
    "phone",
    "address",
    "city",
    "state",
    "zip",
    "dob",
    "fullName",
];

/// Dangerous URL schemes that must never be navigated to.
const BLOCKED_URL_SCHEMES: &[&str] = &[
    "javascript:",
    "data:",
    "file:",
    "blob:",
    "vbscript:",
    "about:",
    "chrome:",
    "chrome-extension:",
];

/// Patterns that should never appear in CSS selectors.
/// These indicate attempts to inject JS event handlers or script content.
const BLOCKED_SELECTOR_PATTERNS: &[&str] = &[
    "javascript:",
    "<script",
    "onerror",
    "onload",
    "onclick",
    "onmouseover",
    "onfocus",
    "onblur",
    "onchange",
    "oninput",
    "onsubmit",
    "onkeydown",
    "onkeyup",
    "onkeypress",
    "onmousedown",
    "onmouseup",
    "ondblclick",
    "oncontextmenu",
    "expression(",
    "url(",
    "import(",
];

/// Validate a full list of playbook steps. Returns Ok(()) or an error describing the problem.
pub fn validate_steps(steps: &[PlaybookStep]) -> Result<(), String> {
    if steps.is_empty() {
        return Err("Playbook must have at least one step.".to_string());
    }
    if steps.len() > MAX_STEPS {
        return Err(format!(
            "Playbook has {} steps, maximum allowed is {}.",
            steps.len(),
            MAX_STEPS
        ));
    }

    for (i, step) in steps.iter().enumerate() {
        let ctx = format!("Step {}", i + 1);
        validate_step(step, &ctx)?;
    }

    Ok(())
}

fn validate_step(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    // Action allowlist
    if !ALLOWED_ACTIONS.contains(&step.action.as_str()) {
        return Err(format!(
            "{}: Unknown action '{}'. Allowed: {}",
            ctx,
            step.action,
            ALLOWED_ACTIONS.join(", ")
        ));
    }

    // Validate selector if present
    if let Some(ref sel) = step.selector {
        validate_selector(sel, ctx)?;
    }

    // Validate value if present
    if let Some(ref val) = step.value {
        validate_value(val, &step.action, ctx)?;
    }

    // Validate description length
    if step.description.len() > MAX_DESCRIPTION_LEN {
        return Err(format!(
            "{}: Description too long ({} chars, max {}).",
            ctx,
            step.description.len(),
            MAX_DESCRIPTION_LEN
        ));
    }

    // Validate instructions length if present
    if let Some(ref instructions) = step.instructions {
        if instructions.len() > MAX_INSTRUCTIONS_LEN {
            return Err(format!(
                "{}: Instructions too long ({} chars, max {}).",
                ctx,
                instructions.len(),
                MAX_INSTRUCTIONS_LEN
            ));
        }
    }

    // Validate profile_key if present
    if let Some(ref key) = step.profile_key {
        if !ALLOWED_PROFILE_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "{}: Unknown profile key '{}'. Allowed: {}",
                ctx,
                key,
                ALLOWED_PROFILE_KEYS.join(", ")
            ));
        }
    }

    // Validate wait_after_ms
    if step.wait_after_ms > MAX_WAIT_MS {
        return Err(format!(
            "{}: wait_after_ms is {} ms, maximum allowed is {} ms.",
            ctx, step.wait_after_ms, MAX_WAIT_MS
        ));
    }

    // Action-specific validation
    match step.action.as_str() {
        "navigate" => validate_navigate_step(step, ctx)?,
        "fill" => validate_fill_step(step, ctx)?,
        "select" => validate_select_step(step, ctx)?,
        "click" | "check" | "scroll_to" | "find_and_click" | "wait_for" => {
            validate_requires_selector(step, ctx)?
        }
        "wait" => validate_wait_step(step, ctx)?,
        _ => {} // captcha, user_prompt, done — no extra validation needed
    }

    Ok(())
}

fn validate_selector(sel: &str, ctx: &str) -> Result<(), String> {
    if sel.is_empty() {
        return Err(format!("{}: Selector is empty.", ctx));
    }
    if sel.len() > MAX_SELECTOR_LEN {
        return Err(format!(
            "{}: Selector too long ({} chars, max {}).",
            ctx,
            sel.len(),
            MAX_SELECTOR_LEN
        ));
    }

    let lower = sel.to_lowercase();
    for pattern in BLOCKED_SELECTOR_PATTERNS {
        if lower.contains(pattern) {
            return Err(format!(
                "{}: Selector contains blocked pattern '{}'.",
                ctx, pattern
            ));
        }
    }

    Ok(())
}

fn validate_value(val: &str, action: &str, ctx: &str) -> Result<(), String> {
    if val.len() > MAX_VALUE_LEN {
        return Err(format!(
            "{}: Value too long ({} chars, max {}).",
            ctx,
            val.len(),
            MAX_VALUE_LEN
        ));
    }

    // For navigate actions, URL validation is done separately
    if action == "navigate" {
        return Ok(());
    }

    // Block script-like content in values
    let lower = val.to_lowercase();
    if lower.contains("<script") || lower.contains("javascript:") {
        return Err(format!(
            "{}: Value contains blocked content.",
            ctx
        ));
    }

    Ok(())
}

fn validate_navigate_step(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    let url = step
        .value
        .as_deref()
        .ok_or_else(|| format!("{}: Navigate step requires a URL value.", ctx))?;

    if url.is_empty() {
        return Err(format!("{}: Navigate URL is empty.", ctx));
    }

    let lower = url.to_lowercase().trim_start().to_string();

    // Block dangerous schemes
    for scheme in BLOCKED_URL_SCHEMES {
        if lower.starts_with(scheme) {
            return Err(format!(
                "{}: Navigate URL uses blocked scheme '{}'. Only http:// and https:// are allowed.",
                ctx, scheme
            ));
        }
    }

    // Must start with http:// or https://
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!(
            "{}: Navigate URL must start with http:// or https://.",
            ctx
        ));
    }

    // Block URLs to localhost/internal networks
    let after_scheme = if lower.starts_with("https://") {
        &lower[8..]
    } else {
        &lower[7..]
    };
    let host = after_scheme.split('/').next().unwrap_or("");
    let host_no_port = host.split(':').next().unwrap_or("");

    if host_no_port == "localhost"
        || host_no_port == "127.0.0.1"
        || host_no_port == "0.0.0.0"
        || host_no_port == "[::1]"
        || host_no_port.starts_with("192.168.")
        || host_no_port.starts_with("10.")
        || host_no_port.starts_with("172.16.")
        || host_no_port.ends_with(".local")
    {
        return Err(format!(
            "{}: Navigate URL points to a local/internal address, which is not allowed.",
            ctx
        ));
    }

    Ok(())
}

fn validate_fill_step(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    validate_requires_selector(step, ctx)?;
    // Fill steps can use profile_key (auto-fill from profile), or no profile_key (manual fill by user)
    Ok(())
}

fn validate_select_step(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    validate_requires_selector(step, ctx)?;
    // Select steps can use profile_key (auto-fill from profile), value (fixed), or neither (manual selection by user)
    Ok(())
}

fn validate_requires_selector(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    if step.selector.is_none() || step.selector.as_deref() == Some("") {
        return Err(format!(
            "{}: '{}' step requires a selector.",
            ctx, step.action
        ));
    }
    Ok(())
}

fn validate_wait_step(step: &PlaybookStep, ctx: &str) -> Result<(), String> {
    // The wait time comes from wait_after_ms which is already capped above,
    // but also check the value field if used for explicit waits
    if let Some(ref val) = step.value {
        if let Ok(ms) = val.parse::<u64>() {
            if ms > MAX_WAIT_MS as u64 {
                return Err(format!(
                    "{}: Wait value is {} ms, maximum allowed is {} ms.",
                    ctx, ms, MAX_WAIT_MS
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlaybookStep;

    fn make_step(action: &str) -> PlaybookStep {
        PlaybookStep {
            position: 1,
            action: action.to_string(),
            selector: Some("#test".to_string()),
            profile_key: None,
            value: None,
            description: "Test step".to_string(),
            instructions: None,
            wait_after_ms: 500,
            optional: false,
        }
    }

    #[test]
    fn rejects_unknown_action() {
        let step = make_step("eval_js");
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_javascript_url() {
        let mut step = make_step("navigate");
        step.value = Some("javascript:alert(1)".to_string());
        step.selector = None;
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_file_url() {
        let mut step = make_step("navigate");
        step.value = Some("file:///etc/passwd".to_string());
        step.selector = None;
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_data_url() {
        let mut step = make_step("navigate");
        step.value = Some("data:text/html,<script>alert(1)</script>".to_string());
        step.selector = None;
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn allows_https_url() {
        let mut step = make_step("navigate");
        step.value = Some("https://example.com/opt-out".to_string());
        step.selector = None;
        assert!(validate_steps(&[step]).is_ok());
    }

    #[test]
    fn rejects_localhost_url() {
        let mut step = make_step("navigate");
        step.value = Some("http://localhost:8080/admin".to_string());
        step.selector = None;
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_selector_with_event_handler() {
        let mut step = make_step("click");
        step.selector = Some("[onerror=\"alert(1)\"]".to_string());
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_too_many_steps() {
        let steps: Vec<PlaybookStep> = (0..101)
            .map(|i| {
                let mut s = make_step("click");
                s.position = i + 1;
                s
            })
            .collect();
        assert!(validate_steps(&steps).is_err());
    }

    #[test]
    fn rejects_excessive_wait() {
        let mut step = make_step("click");
        step.wait_after_ms = 60_000;
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn rejects_unknown_profile_key() {
        let mut step = make_step("fill");
        step.profile_key = Some("ssn".to_string());
        assert!(validate_steps(&[step]).is_err());
    }

    #[test]
    fn allows_valid_fill() {
        let mut step = make_step("fill");
        step.profile_key = Some("firstName".to_string());
        assert!(validate_steps(&[step]).is_ok());
    }

    #[test]
    fn rejects_script_in_value() {
        let mut step = make_step("select");
        step.value = Some("<script>alert(1)</script>".to_string());
        assert!(validate_steps(&[step]).is_err());
    }
}
