use crate::models::Playbook;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};

/// Ed25519 public key for verifying community playbook signatures.
/// Release builds can override via `PLAYBOOK_PUBLIC_KEY` env var at compile time.
static PLAYBOOK_PUBLIC_KEY: std::sync::LazyLock<VerifyingKey> = std::sync::LazyLock::new(|| {
    let key_b64 = option_env!("PLAYBOOK_PUBLIC_KEY")
        .unwrap_or("AsWpThdraJZ589wFqx/wHkFAnl0GY7kRjATEFoaSBCg=");
    let key_bytes = STANDARD.decode(key_b64).expect("PLAYBOOK_PUBLIC_KEY must be valid base64");
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .expect("PLAYBOOK_PUBLIC_KEY must decode to exactly 32 bytes");
    VerifyingKey::from_bytes(&key_array).expect("PLAYBOOK_PUBLIC_KEY must be a valid Ed25519 public key")
});

/// Verify the Ed25519 signature on a community playbook.
///
/// Builds a canonical JSON representation of the steps, then checks the
/// base64-encoded signature against the embedded public key.
pub fn verify_playbook_signature(playbook: &Playbook) -> Result<(), String> {

    let sig_b64 = playbook
        .signature
        .as_deref()
        .ok_or("Community playbook is missing a signature")?;

    let sig_bytes = STANDARD
        .decode(sig_b64)
        .map_err(|e| format!("Invalid signature encoding: {}", e))?;

    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| "Signature must be exactly 64 bytes".to_string())?;

    let signature = Signature::from_bytes(&sig_array);

    // Build canonical JSON: steps sorted by position, each with 9 keys in alphabetical order
    let mut sorted_steps = playbook.steps.clone();
    sorted_steps.sort_by_key(|s| s.position);

    let canonical_steps: Vec<serde_json::Value> = sorted_steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "action": step.action,
                "description": step.description,
                "instructions": step.instructions,
                "optional": step.optional,
                "position": step.position,
                "profile_key": step.profile_key,
                "selector": step.selector,
                "value": step.value,
                "wait_after_ms": step.wait_after_ms
            })
        })
        .collect();

    let canonical_json = serde_json::to_string(&canonical_steps)
        .map_err(|e| format!("Failed to serialize canonical steps: {}", e))?;

    // The server (PHP) escapes forward slashes as \/ in json_encode.
    // Match that behavior so the signed bytes are identical.
    let canonical_json = canonical_json.replace("/", "\\/");

    PLAYBOOK_PUBLIC_KEY
        .verify_strict(canonical_json.as_bytes(), &signature)
        .map_err(|_| "Playbook signature verification failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlaybookStep;

    fn spokeo_steps() -> Vec<PlaybookStep> {
        vec![
            PlaybookStep {
                position: 1,
                action: "user_prompt".to_string(),
                selector: None,
                profile_key: None,
                value: Some("Search for your Profile URL in the search bar at the top of the page.\nProfile URL Example: \"https://www.spokeo.com/Smith-Sample/Houston/TX/p12345678\"".to_string()),
                description: "Search For Profile URL".to_string(),
                instructions: Some("Search for your name in the database using the search bar on the top of the page to get your profile URL.".to_string()),
                wait_after_ms: 1000,
                optional: false,
            },
            PlaybookStep {
                position: 2,
                action: "fill".to_string(),
                selector: Some("input[name=\"url\"]".to_string()),
                profile_key: None,
                value: None,
                description: "Enter URL in URL".to_string(),
                instructions: None,
                wait_after_ms: 500,
                optional: false,
            },
            PlaybookStep {
                position: 3,
                action: "fill".to_string(),
                selector: Some("input[name=\"email\"]".to_string()),
                profile_key: Some("email".to_string()),
                value: None,
                description: "Enter email in Email Address".to_string(),
                instructions: None,
                wait_after_ms: 500,
                optional: false,
            },
            PlaybookStep {
                position: 4,
                action: "captcha".to_string(),
                selector: None,
                profile_key: None,
                value: None,
                description: "Solve CAPTCHA".to_string(),
                instructions: None,
                wait_after_ms: 500,
                optional: false,
            },
            PlaybookStep {
                position: 5,
                action: "click".to_string(),
                selector: Some("#root > div:nth-of-type(2) > div:nth-of-type(2) > div > div > form > div:nth-of-type(4) > button".to_string()),
                profile_key: None,
                value: None,
                description: "Click \"OPT OUT\"".to_string(),
                instructions: None,
                wait_after_ms: 500,
                optional: false,
            },
            PlaybookStep {
                position: 6,
                action: "user_prompt".to_string(),
                selector: None,
                profile_key: None,
                value: Some("Check email for confirmation link and click on it to see confirmation below the form.".to_string()),
                description: "Check Email".to_string(),
                instructions: Some("Check email for link to click on. When you click on the link you should see a verification that it worked below the form.".to_string()),
                wait_after_ms: 1000,
                optional: false,
            },
        ]
    }

    #[test]
    fn test_canonical_json_matches_php() {
        let steps = spokeo_steps();

        let canonical_steps: Vec<serde_json::Value> = steps
            .iter()
            .map(|step| {
                serde_json::json!({
                    "action": step.action,
                    "description": step.description,
                    "instructions": step.instructions,
                    "optional": step.optional,
                    "position": step.position,
                    "profile_key": step.profile_key,
                    "selector": step.selector,
                    "value": step.value,
                    "wait_after_ms": step.wait_after_ms
                })
            })
            .collect();

        let canonical_json = serde_json::to_string(&canonical_steps).unwrap();
        let canonical_json = canonical_json.replace("/", "\\/");

        // Verify first step format matches PHP json_encode output
        assert!(canonical_json.starts_with(r#"[{"action":"user_prompt","description":"Search For Profile URL""#));
        assert!(canonical_json.contains(r#"https:\/\/www.spokeo.com"#));
        assert_eq!(canonical_json.len(), 1640);
    }

    #[test]
    fn test_signature_verification() {
        let playbook = Playbook {
            id: "019c6563-c451-7357-8960-f96adb3d0916".to_string(),
            broker_id: "spokeo".to_string(),
            broker_name: "Spokeo".to_string(),
            title: Some("Admin Created".to_string()),
            version: 1,
            status: "approved".to_string(),
            notes: None,
            steps: spokeo_steps(),
            signature: Some("nP+0GxNFT5r32DwMnwBPjjGrjluwXmSmu40RtnLHj1T2k54DemnZZ+o9IORIpQsDxJoaNhCM0ttZ2g46JcknCQ==".to_string()),
            upvotes: 2,
            downvotes: 0,
            success_count: 0,
            failure_count: 0,
            created_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let result = verify_playbook_signature(&playbook);
        assert!(result.is_ok(), "Signature verification failed: {:?}", result.err());
    }
}
