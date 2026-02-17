export interface Profile {
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  address: string;
  city: string;
  state: string;
  zip: string;
  dob: string;
  alternateEmails: string[];
  alternatePhones: string[];
  previousAddresses: PreviousAddress[];
}

export interface PreviousAddress {
  address: string;
  city: string;
  state: string;
  zip: string;
}

export interface KnownField {
  label: string;
  type: string;
  profile_key: string | null;
}

export interface Broker {
  id: string;
  name: string;
  url: string;
  category: BrokerCategory;
  method: BrokerMethod;
  opt_out_url: string;
  known_fields: KnownField[];
  notes: string;
  requires_verification: string | null;
  relist_days: number | null;
  difficulty: BrokerDifficulty;
  last_verified: string;
}

export type BrokerCategory =
  | "people-search"
  | "marketing"
  | "background-check"
  | "data-aggregator"
  | "identity";

export type BrokerMethod = "web-form" | "email" | "api";

export type BrokerDifficulty = "easy" | "medium" | "hard";

export interface BrokerRegistry {
  version: string;
  brokers: Broker[];
}

export enum BrokerStatus {
  NotStarted = "not_started",
  Submitted = "submitted",
  PendingVerification = "pending_verification",
  Confirmed = "confirmed",
  ReListed = "re_listed",
  Failed = "failed",
}

export interface SubmissionRecord {
  id: string;
  broker_id: string;
  status: string;
  submitted_at: string;
  confirmed_at: string | null;
  next_check_date: string | null;
  error_message: string | null;
  run_id: string;
}

export type RunStatus =
  | "idle"
  | "running"
  | "waiting_for_user"
  | "paused"
  | "completed"
  | "failed";

export interface UserActionRequired {
  type: "solve_captcha" | "verify_email" | "verify_phone" | "manual_step" | "user_prompt" | "step_failed";
  captcha_type?: string;
  message: string;
  description?: string;
  step_description?: string;
  step_position?: number;
  broker_name?: string;
}

export interface OptOutProgress {
  run_id: string;
  broker_id: string;
  broker_name: string;
  status: RunStatus;
  current_step: string;
  brokers_completed: number;
  brokers_total: number;
  action_required: UserActionRequired | null;
  error: string | null;
}

export interface OptOutComplete {
  run_id: string;
  total: number;
  succeeded: number;
  failed: number;
}

// --- Community Playbook types ---

export interface PlaybookStep {
  position: number;
  action: string;
  selector: string | null;
  profile_key: string | null;
  value: string | null;
  description: string;
  instructions: string | null;
  wait_after_ms: number;
  optional: boolean;
}

export interface PlaybookSummary {
  id: string;
  broker_id: string;
  broker_name: string;
  title: string | null;
  version: number;
  notes: string | null;
  steps_count: number;
  upvotes: number;
  downvotes: number;
  success_count: number;
  failure_count: number;
  score: number;
  created_at: string;
}

export interface Playbook {
  id: string;
  broker_id: string;
  broker_name: string;
  title: string | null;
  version: number;
  status: string;
  notes: string | null;
  steps: PlaybookStep[];
  signature: string | null;
  upvotes: number;
  downvotes: number;
  success_count: number;
  failure_count: number;
  created_at: string;
}

export interface LocalPlaybook {
  id: string;
  brokerId: string;
  brokerName: string;
  title: string | null;
  notes: string | null;
  steps: PlaybookStep[];
  createdAt: string;
  updatedAt: string;
  submittedAt: string | null;
}

export interface RecordedAction {
  action: string;
  selector: string | null;
  profile_key: string | null;
  value: string | null;
  url: string | null;
  element_text: string | null;
  label: string | null;
  timestamp: number;
}

// --- Submission Tracker types ---

export interface TrackedSubmission {
  playbook_id: string;
  broker_id: string;
  broker_name: string;
  status: string;
  submitted_at: string;
  local_playbook_id: string | null;
}

// --- Changelog types ---

export interface ChangelogEntry {
  version: string;
  date: string;
  description: string;
}

// --- Playbook Report types ---

export interface PlaybookReportEntry {
  outcome: string;
  failure_step: number | null;
  error_message: string | null;
  app_version: string;
  created_at: string;
}

export type RecordingStatus = "idle" | "recording" | "reviewing" | "submitting";

export interface PlaybookSubmission {
  broker_id: string;
  broker_name: string;
  title: string | null;
  notes: string | null;
  steps: PlaybookStep[];
}
