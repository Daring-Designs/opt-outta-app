use crate::models::{ApiEnvelope, Playbook, PlaybookReport, PlaybookSubmission, PlaybookSubmitResponse, PlaybookSummary};
use ed25519_dalek::{SigningKey, Signer};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::time::{SystemTime, UNIX_EPOCH};

const API_BASE: &str = "https://opt-outta.com/api/v1";

/// Ed25519 signing key, loaded from env at compile time.
static SIGNING_KEY: std::sync::LazyLock<SigningKey> = std::sync::LazyLock::new(|| {
    let key_b64 = env!("API_PRIVATE_KEY");
    let key_bytes = STANDARD.decode(key_b64).expect("API_PRIVATE_KEY must be valid base64");
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .expect("API_PRIVATE_KEY must decode to exactly 32 bytes");
    SigningKey::from_bytes(&key_array)
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
    // URL is always API_BASE + suffix, so strip the origin
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
async fn signed_get(url: &str) -> Result<reqwest::Response, String> {
    let path = url_path(url);
    let (signature, timestamp) = sign_request("GET", &path, "");

    reqwest::Client::new()
        .get(url)
        .header("X-Signature", signature)
        .header("X-Timestamp", timestamp)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))
}

/// Send a signed POST request with a JSON body and return the response.
async fn signed_post(url: &str, body: &str) -> Result<reqwest::Response, String> {
    let path = url_path(url);
    let (signature, timestamp) = sign_request("POST", &path, body);

    reqwest::Client::new()
        .post(url)
        .header("X-Signature", signature)
        .header("X-Timestamp", timestamp)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))
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
    let url = format!("{}/playbooks?broker_id={}&sort=best&limit=1", API_BASE, broker_id);

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
    let url = format!("{}/playbooks/{}", API_BASE, id);

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
    let url = format!("{}/playbooks?broker_id={}&sort=best&limit=10", API_BASE, broker_id);

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
    let url = format!("{}/playbooks", API_BASE);
    let body = serde_json::to_string(submission)
        .map_err(|e| format!("Failed to serialize submission: {}", e))?;

    let response = signed_post(&url, &body).await?;

    if !response.status().is_success() {
        let status = response.status();
        let resp_body = response.text().await.unwrap_or_default();
        return Err(format!("Playbook submit error ({}): {}", status, resp_body));
    }

    let envelope: ApiEnvelope<PlaybookSubmitResponse> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse submit response: {}", e))?;

    Ok(envelope.data)
}

/// Vote on a playbook (up or down).
pub async fn vote_playbook(id: &str, vote: &str) -> Result<(), String> {
    let url = format!("{}/playbooks/{}/vote", API_BASE, id);
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

/// Report the outcome of running a playbook.
pub async fn report_outcome(playbook_id: &str, report: &PlaybookReport) -> Result<(), String> {
    let url = format!("{}/playbooks/{}/report", API_BASE, playbook_id);
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
