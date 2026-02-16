use crate::models::Playbook;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, VerifyingKey};

/// Ed25519 public key for verifying community playbook signatures, embedded at
/// compile time via `PLAYBOOK_PUBLIC_KEY` env var. Falls back to a dummy key
/// for local dev builds (signature verification will always fail).
static PLAYBOOK_PUBLIC_KEY: std::sync::LazyLock<VerifyingKey> = std::sync::LazyLock::new(|| {
    let key_b64 = option_env!("PLAYBOOK_PUBLIC_KEY")
        .unwrap_or("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
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

    // Build canonical JSON: steps sorted by position, each with 8 keys in alphabetical order
    let mut sorted_steps = playbook.steps.clone();
    sorted_steps.sort_by_key(|s| s.position);

    let canonical_steps: Vec<serde_json::Value> = sorted_steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "action": step.action,
                "description": step.description,
                "optional": step.optional,
                "position": step.position,
                "profile_key": step.profile_key,
                "selector": step.selector,
                "value": step.value,
                "wait_after_ms": null
            })
        })
        .collect();

    let canonical_json = serde_json::to_string(&canonical_steps)
        .map_err(|e| format!("Failed to serialize canonical steps: {}", e))?;

    PLAYBOOK_PUBLIC_KEY
        .verify_strict(canonical_json.as_bytes(), &signature)
        .map_err(|_| "Playbook signature verification failed".to_string())
}
