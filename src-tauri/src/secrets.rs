use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

const SERVICE_NAME: &str = "opt-outta";
const SECRETS_ENTRY: &str = "secrets";

// Legacy entries (for migration from separate keychain items)
const LEGACY_ENCRYPTION_KEY_ENTRY: &str = "encryption-key";

#[derive(Serialize, Deserialize)]
struct StoredSecrets {
    encryption_key: String, // base64-encoded AES-256 key
}

struct Inner {
    encryption_key: Vec<u8>,
    loaded: bool,
}

/// In-memory cache for secrets backed by a single OS keychain entry.
/// Call `load()` once at startup — all subsequent reads come from memory.
pub struct SecretsCache(Mutex<Inner>);

impl SecretsCache {
    pub fn new() -> Self {
        Self(Mutex::new(Inner {
            encryption_key: Vec::new(),
            loaded: false,
        }))
    }

    /// Load secrets from keychain into memory. Migrates from legacy
    /// per-secret entries to a single combined entry on first run.
    pub fn load(&self) -> Result<(), String> {
        let mut inner = self.0.lock().unwrap();
        if inner.loaded {
            return Ok(());
        }

        let entry = keyring::Entry::new(SERVICE_NAME, SECRETS_ENTRY)
            .map_err(|e| e.to_string())?;

        let stored = match entry.get_password() {
            Ok(json) => serde_json::from_str::<StoredSecrets>(&json)
                .map_err(|e| format!("Failed to parse secrets: {}", e))?,
            Err(keyring::Error::NoEntry) => migrate_legacy(&entry)?,
            Err(e) => return Err(e.to_string()),
        };

        inner.encryption_key = BASE64
            .decode(&stored.encryption_key)
            .map_err(|e| format!("Failed to decode encryption key: {}", e))?;
        inner.loaded = true;
        Ok(())
    }

    pub fn get_encryption_key(&self) -> Result<Vec<u8>, String> {
        let inner = self.0.lock().unwrap();
        if !inner.loaded {
            return Err("Secrets not loaded".to_string());
        }
        Ok(inner.encryption_key.clone())
    }
}

/// Migrate from legacy separate keychain entries to a single combined entry.
fn migrate_legacy(new_entry: &keyring::Entry) -> Result<StoredSecrets, String> {
    // Try to read legacy encryption key
    let enc_key_b64 = {
        let legacy =
            keyring::Entry::new(SERVICE_NAME, LEGACY_ENCRYPTION_KEY_ENTRY)
                .map_err(|e| e.to_string())?;
        match legacy.get_password() {
            Ok(key) => key,
            Err(keyring::Error::NoEntry) => {
                // Fresh install — generate new key
                let mut key = vec![0u8; 32];
                rand::thread_rng().fill_bytes(&mut key);
                BASE64.encode(&key)
            }
            Err(e) => return Err(e.to_string()),
        }
    };

    let stored = StoredSecrets {
        encryption_key: enc_key_b64,
    };

    // Save to new combined entry
    let json = serde_json::to_string(&stored).map_err(|e| e.to_string())?;
    new_entry
        .set_password(&json)
        .map_err(|e| e.to_string())?;

    Ok(stored)
}
