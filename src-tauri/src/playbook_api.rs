use crate::models::{ApiEnvelope, BrokerRegistry, ChangelogEntry, Playbook, PlaybookReport, PlaybookReportEntry, PlaybookSubmission, PlaybookSubmitResponse, PlaybookSummary, RegistryVersionResponse};
use ed25519_dalek::{SigningKey, Signer};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PRODUCTION_API_BASE: &str = "https://opt-outta.com/api/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Sandbox configuration.
/// Dev builds use sandbox by default. Release builds use production unless
/// `USE_SANDBOX=1` is set at compile time.
const SANDBOX_API_URL: &str = "https://sandbox.opt-outta.com/api/v1";
const SANDBOX_TOKEN: &str = "UCnGZbpWZoqNxlfDTTeKEbXRjiE+VJISPjsl6Bp/qDE=";
const USE_SANDBOX: bool = if cfg!(debug_assertions) {
    option_env!("USE_PRODUCTION").is_none()
} else {
    option_env!("USE_SANDBOX").is_some()
};

fn api_base() -> &'static str {
    if USE_SANDBOX { SANDBOX_API_URL } else { PRODUCTION_API_BASE }
}

/// Ed25519 signing key, embedded at compile time via `API_PRIVATE_KEY` env var.
/// Falls back to a dummy key for local dev builds (API calls will be rejected by the server).
static SIGNING_KEY: std::sync::LazyLock<SigningKey> = std::sync::LazyLock::new(|| {
    let key_b64 = option_env!("API_PRIVATE_KEY")
        .unwrap_or("mDtbMauvCa/sJUI1HAQOLRPGqCg+D09JDI4g6AFnML6N7jL91TAk/LCAXW1ahl8AgYhf+7T6vr7XvlE5Df5Y0g==");
    let key_bytes = STANDARD.decode(key_b64).expect("API_PRIVATE_KEY must be valid base64");
    // Sodium secret keys are 64 bytes (32-byte seed + 32-byte public key).
    // ed25519-dalek expects just the 32-byte seed.
    let seed: [u8; 32] = key_bytes[..32]
        .try_into()
        .expect("API_PRIVATE_KEY must be at least 32 bytes");
    SigningKey::from_bytes(&seed)
});

// ---------------------------------------------------------------------------
// Ed25519 request signing
// ---------------------------------------------------------------------------

/// Compute the X-Signature and X-Timestamp headers for a request.
fn sign_request(method: &str, path: &str, body: &str) -> (String, String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let string_to_sign = format!("{}\n{}\n{}\n{}", timestamp, method, path, body);

    let signature = SIGNING_KEY.sign(string_to_sign.as_bytes());
    let sig_b64 = STANDARD.encode(signature.to_bytes());

    (sig_b64, timestamp)
}

/// Extract the path (+ query string) from a full URL.
fn url_path(url: &str) -> String {
    // URL is always api_base + suffix, so strip the origin
    if let Some(pos) = url.find("/api/") {
        url[pos..].to_string()
    } else {
        // Fallback: parse properly
        match reqwest::Url::parse(url) {
            Ok(parsed) => {
                let mut path = parsed.path().to_string();
                if let Some(q) = parsed.query() {
                    path.push('?');
                    path.push_str(q);
                }
                path
            }
            Err(_) => url.to_string(),
        }
    }
}

/// Send a signed GET request and return the response.
/// In sandbox mode, uses bearer token auth instead of Ed25519.
async fn signed_get(url: &str) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut req = client.get(url);

    if USE_SANDBOX {
        req = req.header("Authorization", format!("Bearer {}", SANDBOX_TOKEN));
    } else {
        let path = url_path(url);
        let (signature, timestamp) = sign_request("GET", &path, "");
        req = req.header("X-Signature", signature).header("X-Timestamp", timestamp);
    }

    req.send().await.map_err(|e| format!("Request failed: {}", e))
}

/// Send a signed POST request with a JSON body and return the response.
/// In sandbox mode, uses bearer token auth instead of Ed25519.
async fn signed_post(url: &str, body: &str) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body.to_string());

    if USE_SANDBOX {
        req = req.header("Authorization", format!("Bearer {}", SANDBOX_TOKEN));
    } else {
        let path = url_path(url);
        let (signature, timestamp) = sign_request("POST", &path, body);
        req = req.header("X-Signature", signature).header("X-Timestamp", timestamp);
    }

    req.send().await.map_err(|e| format!("Request failed: {}", e))
}

// ---------------------------------------------------------------------------
// Device ID
// ---------------------------------------------------------------------------

/// Generate an anonymous device ID from hostname + a salt.
pub fn get_device_id() -> String {
    use sha2::Digest;
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let mut hasher = sha2::Sha256::new();
    hasher.update(hostname.as_bytes());
    hasher.update(b"opt-outta-device-salt-v1");
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

/// Fetch the best approved playbook for a broker, if one exists.
pub async fn fetch_best_playbook(broker_id: &str) -> Result<Option<Playbook>, String> {
    let url = format!("{}/playbooks?broker_id={}&sort=best&limit=1", api_base(), broker_id);

    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Playbook API error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<Vec<PlaybookSummary>> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse playbook list: {}", e))?;

    let summary = match envelope.data.first() {
        Some(s) => s,
        None => return Ok(None),
    };

    fetch_playbook_detail(&summary.id).await.map(Some)
}

/// Fetch a single playbook with all steps.
pub async fn fetch_playbook_detail(id: &str) -> Result<Playbook, String> {
    let url = format!("{}/playbooks/{}", api_base(), id);

    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Playbook detail error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<Playbook> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse playbook detail: {}", e))?;

    Ok(envelope.data)
}

/// Fetch list of playbook summaries for a broker.
pub async fn fetch_playbooks(broker_id: &str) -> Result<Vec<PlaybookSummary>, String> {
    let url = format!("{}/playbooks?broker_id={}&sort=best&limit=10", api_base(), broker_id);

    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Playbook list error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<Vec<PlaybookSummary>> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse playbook list: {}", e))?;

    Ok(envelope.data)
}

/// Submit a new recorded playbook.
pub async fn submit_playbook(submission: &PlaybookSubmission) -> Result<PlaybookSubmitResponse, String> {
    let url = format!("{}/playbooks", api_base());
    let body = serde_json::to_string(submission)
        .map_err(|e| format!("Failed to serialize submission: {}", e))?;

    let response = signed_post(&url, &body).await?;

    let status = response.status();
    let resp_body = response.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Playbook submit error ({}): {}", status, resp_body));
    }

    if resp_body.trim_start().starts_with('<') {
        return Err("Server returned HTML instead of JSON — the submit endpoint may not be deployed yet.".to_string());
    }

    let envelope: ApiEnvelope<PlaybookSubmitResponse> = serde_json::from_str(&resp_body)
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    Ok(envelope.data)
}

/// Vote on a playbook (up or down).
pub async fn vote_playbook(id: &str, vote: &str) -> Result<(), String> {
    let url = format!("{}/playbooks/{}/vote", api_base(), id);
    let device_id = get_device_id();

    let body_value = serde_json::json!({
        "device_id": device_id,
        "vote": vote,
    });
    let body = serde_json::to_string(&body_value)
        .map_err(|e| format!("Failed to serialize vote: {}", e))?;

    let response = signed_post(&url, &body).await?;

    if !response.status().is_success() {
        let status = response.status();
        let resp_body = response.text().await.unwrap_or_default();
        return Err(format!("Vote error ({}): {}", status, resp_body));
    }

    Ok(())
}

/// Check the current status of a submitted playbook.
pub async fn check_playbook_status(id: &str) -> Result<String, String> {
    let url = format!("{}/playbooks/{}", api_base(), id);

    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        return Err(format!("Status check failed ({})", response.status()));
    }

    let envelope: ApiEnvelope<Playbook> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse playbook: {}", e))?;

    Ok(envelope.data.status)
}

/// Suggest a new broker to be added to the registry.
pub async fn suggest_broker(name: &str, url: &str, notes: &str) -> Result<(), String> {
    let api_url = format!("{}/broker-suggestions", api_base());
    let device_id = get_device_id();

    let body_value = serde_json::json!({
        "device_id": device_id,
        "name": name,
        "url": url,
        "notes": notes,
    });
    let body = serde_json::to_string(&body_value)
        .map_err(|e| format!("Failed to serialize suggestion: {}", e))?;

    let response = signed_post(&api_url, &body).await?;

    if !response.status().is_success() {
        let status = response.status();
        let resp_body = response.text().await.unwrap_or_default();
        return Err(format!("Suggestion error ({}): {}", status, resp_body));
    }

    Ok(())
}

/// Fetch the current registry version from the API.
pub async fn fetch_registry_version() -> Result<String, String> {
    let url = format!("{}/registry/version", api_base());
    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Registry version error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<RegistryVersionResponse> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse registry version: {}", e))?;

    Ok(envelope.data.version)
}

/// Fetch the full broker registry from the API.
pub async fn fetch_registry() -> Result<BrokerRegistry, String> {
    let url = format!("{}/registry", api_base());
    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Registry fetch error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<BrokerRegistry> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse registry: {}", e))?;

    Ok(envelope.data)
}

/// Fetch the app changelog.
pub async fn fetch_changelog() -> Result<Vec<ChangelogEntry>, String> {
    let url = format!("{}/changelog", api_base());
    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Changelog error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<Vec<ChangelogEntry>> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse changelog: {}", e))?;

    Ok(envelope.data)
}

/// Fetch community execution reports for a playbook.
pub async fn fetch_playbook_reports(playbook_id: &str) -> Result<Vec<PlaybookReportEntry>, String> {
    let url = format!("{}/playbooks/{}/reports", api_base(), playbook_id);
    let response = signed_get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Playbook reports error ({}): {}", status, body));
    }

    let envelope: ApiEnvelope<Vec<PlaybookReportEntry>> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse playbook reports: {}", e))?;

    Ok(envelope.data)
}

/// Report the outcome of running a playbook.
pub async fn report_outcome(playbook_id: &str, report: &PlaybookReport) -> Result<(), String> {
    let url = format!("{}/playbooks/{}/report", api_base(), playbook_id);
    let body = serde_json::to_string(report)
        .map_err(|e| format!("Failed to serialize report: {}", e))?;

    let response = signed_post(&url, &body).await?;

    if !response.status().is_success() {
        let status = response.status();
        let resp_body = response.text().await.unwrap_or_default();
        return Err(format!("Report error ({}): {}", status, resp_body));
    }

    Ok(())
}
