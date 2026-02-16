use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "lastName")]
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub dob: String,
    #[serde(rename = "alternateEmails")]
    pub alternate_emails: Vec<String>,
    #[serde(rename = "alternatePhones")]
    pub alternate_phones: Vec<String>,
    #[serde(rename = "previousAddresses")]
    pub previous_addresses: Vec<PreviousAddress>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreviousAddress {
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnownField {
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub profile_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Broker {
    pub id: String,
    pub name: String,
    pub url: String,
    pub category: String,
    pub method: String,
    pub opt_out_url: String,
    pub known_fields: Vec<KnownField>,
    pub notes: String,
    pub requires_verification: Option<String>,
    pub relist_days: Option<u32>,
    pub difficulty: String,
    pub last_verified: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrokerRegistry {
    pub version: String,
    pub brokers: Vec<Broker>,
}

// --- Phase 2: Opt-out automation types ---

/// A form field extracted from a web page (no PII — only labels/structure)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct FormField {
    pub selector: String,
    pub tag: String,
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub name: Option<String>,
    pub id: Option<String>,
    pub required: bool,
    pub options: Option<Vec<String>>,
    pub visible: bool,
}

/// A form structure extracted from a page
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct FormStructure {
    pub selector: String,
    pub action: Option<String>,
    pub method: Option<String>,
    pub fields: Vec<FormField>,
}

/// A button on the page
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ButtonInfo {
    pub selector: String,
    pub text: String,
    #[serde(rename = "type")]
    pub button_type: Option<String>,
    pub visible: bool,
}

/// Full page structure for browser extraction
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct PageStructure {
    pub url: String,
    pub title: String,
    pub forms: Vec<FormStructure>,
    pub buttons: Vec<ButtonInfo>,
    pub text_blocks: Vec<String>,
    pub has_captcha: bool,
}

/// An individual action in a playbook step
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "action")]
pub enum FormAction {
    #[serde(rename = "fill")]
    Fill {
        selector: String,
        profile_key: Option<String>,
        value: Option<String>,
        transform: Option<String>,
    },
    #[serde(rename = "select")]
    Select {
        selector: String,
        value: String,
    },
    #[serde(rename = "check")]
    Check {
        selector: String,
        checked: bool,
    },
    #[serde(rename = "click")]
    Click {
        selector: String,
    },
    #[serde(rename = "wait")]
    Wait {
        milliseconds: u64,
    },
    #[serde(rename = "captcha")]
    Captcha {
        message: Option<String>,
    },
    #[serde(rename = "navigate")]
    Navigate {
        url: String,
    },
    #[serde(rename = "wait_for")]
    WaitFor {
        selector: String,
        timeout_ms: Option<u64>,
    },
    #[serde(rename = "scroll_to")]
    ScrollTo {
        selector: String,
    },
    #[serde(rename = "find_and_click")]
    FindAndClick {
        selector: String,
        profile_key: String,
    },
    #[serde(rename = "done")]
    Done {
        message: Option<String>,
    },
    #[serde(rename = "user_prompt")]
    UserPrompt {
        message: String,
    },
    #[serde(rename = "manual_fill")]
    ManualFill {
        selector: String,
        message: String,
    },
    #[serde(rename = "manual_select")]
    ManualSelect {
        selector: String,
        message: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// Current status of an opt-out run
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Idle,
    Running,
    WaitingForUser,
    Paused,
    Completed,
    Failed,
}

/// What the user needs to do
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum UserActionRequired {
    #[serde(rename = "solve_captcha")]
    SolveCaptcha { captcha_type: Option<String>, message: String },
    #[serde(rename = "verify_email")]
    VerifyEmail { message: String },
    #[serde(rename = "verify_phone")]
    VerifyPhone { message: String },
    #[serde(rename = "manual_step")]
    ManualStep { message: String },
    #[serde(rename = "user_prompt")]
    UserPrompt { message: String, description: Option<String> },
}

/// Status of an individual broker submission
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerSubmissionStatus {
    Submitted,
    PendingVerification,
    Confirmed,
    Failed,
    ReListed,
}

/// A single opt-out submission record
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubmissionRecord {
    pub id: String,
    pub broker_id: String,
    pub status: BrokerSubmissionStatus,
    pub submitted_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub next_check_date: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub run_id: String,
}

/// Full submission history
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SubmissionHistory {
    pub records: Vec<SubmissionRecord>,
}

/// Event payload for frontend progress updates
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptOutProgress {
    pub run_id: String,
    pub broker_id: String,
    pub broker_name: String,
    pub status: RunStatus,
    pub current_step: String,
    pub brokers_completed: usize,
    pub brokers_total: usize,
    pub action_required: Option<UserActionRequired>,
    pub error: Option<String>,
}

// --- Community Playbook types ---

/// A single step in a community playbook
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookStep {
    pub position: u32,
    pub action: String,
    pub selector: Option<String>,
    pub profile_key: Option<String>,
    pub value: Option<String>,
    pub description: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default = "default_wait_after")]
    pub wait_after_ms: u32,
    #[serde(default)]
    pub optional: bool,
}

fn default_wait_after() -> u32 {
    500
}

/// Full playbook with steps (from GET /playbooks/{id})
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Playbook {
    pub id: String,
    pub broker_id: String,
    pub broker_name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub version: u32,
    pub status: String,
    pub notes: Option<String>,
    pub steps: Vec<PlaybookStep>,
    pub signature: Option<String>,
    pub upvotes: u32,
    pub downvotes: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub created_at: String,
}

/// Playbook summary (from GET /playbooks list).
/// The API returns steps + signature; we deserialize them for verification
/// but skip them when serializing to the frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookSummary {
    pub id: String,
    pub broker_id: String,
    pub broker_name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub version: u32,
    pub notes: Option<String>,
    pub steps_count: u32,
    pub upvotes: u32,
    pub downvotes: u32,
    pub success_count: u32,
    pub failure_count: u32,
    pub score: i32,
    pub created_at: String,
    /// Used for signature verification; not sent to the frontend.
    #[serde(default, skip_serializing)]
    pub signature: Option<String>,
    /// Used for signature verification; not sent to the frontend.
    #[serde(default, skip_serializing)]
    pub steps: Vec<PlaybookStep>,
}

/// Standard API response envelope
#[derive(Debug, Deserialize)]
pub struct ApiEnvelope<T> {
    pub data: T,
    #[allow(dead_code)]
    pub meta: Option<serde_json::Value>,
}

/// Payload for submitting a new playbook
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookSubmission {
    pub broker_id: String,
    pub broker_name: String,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub steps: Vec<PlaybookStep>,
}

/// Response from POST /playbooks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookSubmitResponse {
    pub id: String,
    pub status: String,
    pub message: String,
}

/// Payload for reporting playbook execution outcome
#[derive(Debug, Serialize)]
pub struct PlaybookReport {
    pub device_id: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_step: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub app_version: String,
}

// --- Local Playbook types ---

/// A locally saved playbook draft
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalPlaybook {
    pub id: String,
    #[serde(rename = "brokerId")]
    pub broker_id: String,
    #[serde(rename = "brokerName")]
    pub broker_name: String,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub steps: Vec<PlaybookStep>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "submittedAt", default)]
    pub submitted_at: Option<String>,
}

/// Storage wrapper for local playbooks
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LocalPlaybookStore {
    pub playbooks: Vec<LocalPlaybook>,
}

// --- Submission Tracker types ---

/// Tracks a playbook the user submitted to the community
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrackedSubmission {
    pub playbook_id: String,
    pub broker_id: String,
    pub broker_name: String,
    pub status: String,
    pub submitted_at: String,
    #[serde(default)]
    pub local_playbook_id: Option<String>,
}

/// Storage wrapper for tracked submissions
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SubmissionTrackerStore {
    pub submissions: Vec<TrackedSubmission>,
}

// --- Registry Sync types ---

/// Response from GET /registry/version
#[derive(Debug, Deserialize)]
pub struct RegistryVersionResponse {
    pub version: String,
}

// --- Changelog types ---

/// A single changelog entry from GET /changelog
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub description: String,
}

// --- Playbook Report types ---

/// A single execution report from GET /playbooks/{id}/reports
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaybookReportEntry {
    pub outcome: String,
    pub failure_step: Option<u32>,
    pub error_message: Option<String>,
    pub app_version: String,
    pub created_at: String,
}

/// A single recorded user action during recording mode
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordedAction {
    pub action: String,
    pub selector: Option<String>,
    pub profile_key: Option<String>,
    pub value: Option<String>,
    pub url: Option<String>,
    pub element_text: Option<String>,
    pub label: Option<String>,
    pub timestamp: u64,
}
