import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  RecordedAction,
  PlaybookStep,
  PlaybookSummary,
  Playbook,
  LocalPlaybook,
  RecordingStatus,
  PlaybookSubmission,
  TrackedSubmission,
} from "../types";

export const usePlaybooksStore = defineStore("playbooks", () => {
  // Recording state
  const recordingStatus = ref<RecordingStatus>("idle");
  const recordingBrokerId = ref<string | null>(null);
  const recordingBrokerName = ref<string | null>(null);
  const recordedActions = ref<RecordedAction[]>([]);
  const editableSteps = ref<PlaybookStep[]>([]);
  const playbookTitle = ref<string | null>(null);

  // Live polling state
  const seenActionCount = ref(0);
  const pollingInterval = ref<ReturnType<typeof setInterval> | null>(null);

  // Playbook browsing state
  const playbookCache = ref<Map<string, PlaybookSummary[]>>(new Map());
  const selectedPlaybook = ref<Playbook | null>(null);
  const loadingPlaybooks = ref(false);

  // Local playbook state
  const localPlaybooks = ref<LocalPlaybook[]>([]);
  const editingLocalId = ref<string | null>(null);

  // Submission tracker state
  const trackedSubmissions = ref<TrackedSubmission[]>([]);
  const hasSubmitted = computed(() => trackedSubmissions.value.length > 0);
  const hasApproved = computed(() =>
    trackedSubmissions.value.some((s) => s.status === "approved")
  );

  const isRecording = computed(() => recordingStatus.value === "recording");
  const isReviewing = computed(() => recordingStatus.value === "reviewing");

  // --- Recording ---

  async function startRecording(brokerId: string, brokerName: string, optOutUrl: string) {
    await invoke("start_recording", {
      brokerId,
      brokerName,
      optOutUrl,
    });
    recordingBrokerId.value = brokerId;
    recordingBrokerName.value = brokerName;
    recordedActions.value = [];
    editableSteps.value = [];
    seenActionCount.value = 0;
    recordingStatus.value = "recording";
    pollingInterval.value = setInterval(pollActions, 1500);
  }

  async function stopRecording() {
    if (pollingInterval.value) {
      clearInterval(pollingInterval.value);
      pollingInterval.value = null;
    }
    const actions = await invoke<RecordedAction[]>("stop_recording");
    recordedActions.value = actions;
    // Reconcile: append any actions we haven't seen yet
    if (actions.length > seenActionCount.value) {
      const unseen = actions.slice(seenActionCount.value);
      const newSteps = convertActionsToSteps(unseen);
      // Re-number appended steps to continue from existing
      const basePos = editableSteps.value.length;
      newSteps.forEach((s, i) => (s.position = basePos + i + 1));
      editableSteps.value = [...editableSteps.value, ...newSteps];
      seenActionCount.value = actions.length;
    }
    recordingStatus.value = "reviewing";
  }

  async function markCaptcha() {
    await invoke("mark_captcha_step");
  }

  async function markUserPrompt() {
    await invoke("mark_user_prompt_step");
  }

  async function pollActions() {
    try {
      const actions = await invoke<RecordedAction[]>("get_recorded_actions");
      if (actions.length > seenActionCount.value) {
        const unseen = actions.slice(seenActionCount.value);
        const newSteps = convertActionsToSteps(unseen);
        const basePos = editableSteps.value.length;
        newSteps.forEach((s, i) => (s.position = basePos + i + 1));
        editableSteps.value = [...editableSteps.value, ...newSteps];
        seenActionCount.value = actions.length;
      }
    } catch {
      // Recording may have ended between polls — ignore
    }
  }

  function convertActionsToSteps(actions: RecordedAction[]): PlaybookStep[] {
    return actions.map((a, i) => ({
      position: i + 1,
      action: a.action,
      selector: a.selector,
      profile_key: a.profile_key,
      value: a.value ?? a.url,
      description: generateDescription(a),
      wait_after_ms: 500,
      optional: false,
    }));
  }

  function generateDescription(action: RecordedAction): string {
    switch (action.action) {
      case "navigate":
        return `Go to ${action.url || action.value || "page"}`;
      case "fill":
        return `Enter ${action.profile_key || action.label || "value"} in ${action.label || action.selector || "field"}`;
      case "select":
        return `Select ${action.profile_key || "option"} in ${action.label || action.selector || "dropdown"}`;
      case "check":
        return `Toggle checkbox ${action.label || action.selector || ""}`;
      case "click":
        return `Click "${action.element_text || action.selector || "element"}"`;
      case "captcha":
        return "Solve CAPTCHA";
      case "user_prompt":
        return action.label || action.element_text || "Manual step";
      case "wait":
        return "Wait for page";
      case "wait_for":
        return `Wait for ${action.selector || "element"}`;
      default:
        return action.action;
    }
  }

  // --- Step editing ---

  function reorderStep(from: number, to: number) {
    const steps = [...editableSteps.value];
    const [moved] = steps.splice(from, 1);
    steps.splice(to, 0, moved);
    steps.forEach((s, i) => (s.position = i + 1));
    editableSteps.value = steps;
  }

  function deleteStep(index: number) {
    const steps = [...editableSteps.value];
    steps.splice(index, 1);
    steps.forEach((s, i) => (s.position = i + 1));
    editableSteps.value = steps;
  }

  function addUserPromptStep() {
    const steps = [...editableSteps.value];
    steps.push({
      position: steps.length + 1,
      action: "user_prompt",
      selector: null,
      profile_key: null,
      value: null,
      description: "",
      wait_after_ms: 1000,
      optional: false,
    });
    editableSteps.value = steps;
  }

  function updateStep(index: number, updates: Partial<PlaybookStep>) {
    const steps = [...editableSteps.value];
    steps[index] = { ...steps[index], ...updates };
    editableSteps.value = steps;
  }

  // --- Validation ---

  const ALLOWED_ACTIONS = new Set([
    "navigate", "fill", "select", "check", "click", "wait",
    "wait_for", "scroll_to", "find_and_click", "captcha", "user_prompt", "done",
  ]);

  const ALLOWED_PROFILE_KEYS = new Set([
    "firstName", "lastName", "email", "phone", "address",
    "city", "state", "zip", "dob", "fullName",
  ]);

  const BLOCKED_URL_SCHEMES = [
    "javascript:", "data:", "file:", "blob:", "vbscript:",
    "about:", "chrome:", "chrome-extension:",
  ];

  const BLOCKED_SELECTOR_PATTERNS = [
    "javascript:", "<script", "onerror", "onload", "onclick",
    "onmouseover", "onfocus", "onblur", "onchange", "oninput",
    "onsubmit", "onkeydown", "onkeyup", "onkeypress",
    "expression(", "url(", "import(",
  ];

  function validateSteps(steps: PlaybookStep[]): string | null {
    if (steps.length === 0) return "Playbook must have at least one step.";
    if (steps.length > 100) return `Too many steps (${steps.length}). Maximum is 100.`;

    for (let i = 0; i < steps.length; i++) {
      const step = steps[i];
      const ctx = `Step ${i + 1}`;

      if (!ALLOWED_ACTIONS.has(step.action)) {
        return `${ctx}: Unknown action '${step.action}'.`;
      }

      if (step.selector) {
        if (step.selector.length > 500) return `${ctx}: Selector too long.`;
        const lower = step.selector.toLowerCase();
        for (const pat of BLOCKED_SELECTOR_PATTERNS) {
          if (lower.includes(pat)) return `${ctx}: Selector contains blocked pattern '${pat}'.`;
        }
      }

      if (step.value && step.value.length > 2000) return `${ctx}: Value too long.`;

      if (step.action === "navigate") {
        const url = step.value;
        if (!url) return `${ctx}: Navigate step requires a URL.`;
        const lower = url.toLowerCase().trimStart();
        for (const scheme of BLOCKED_URL_SCHEMES) {
          if (lower.startsWith(scheme)) return `${ctx}: URL uses blocked scheme '${scheme}'.`;
        }
        if (!lower.startsWith("http://") && !lower.startsWith("https://")) {
          return `${ctx}: URL must start with http:// or https://.`;
        }
      }

      if (step.profile_key && !ALLOWED_PROFILE_KEYS.has(step.profile_key)) {
        return `${ctx}: Unknown profile key '${step.profile_key}'.`;
      }

      if (step.wait_after_ms > 30000) {
        return `${ctx}: Wait time too long (${step.wait_after_ms}ms, max 30000ms).`;
      }

      if (step.description.length > 500) return `${ctx}: Description too long.`;
    }

    return null;
  }

  // --- Submit ---

  function checkForPII(steps: PlaybookStep[]): string | null {
    const emailRe = /\S+@\S+\.\S+/;
    const phoneRe = /\b\d{3}[-.]?\d{3}[-.]?\d{4}\b/;
    const ssnRe = /\b\d{3}-\d{2}-\d{4}\b/;

    for (const step of steps) {
      for (const field of [step.value, step.description]) {
        if (!field) continue;
        if (emailRe.test(field)) return "An email address was detected in the steps. Please remove it before submitting.";
        if (phoneRe.test(field)) return "A phone number was detected in the steps. Please remove it before submitting.";
        if (ssnRe.test(field)) return "An SSN pattern was detected in the steps. Please remove it before submitting.";
      }
    }
    return null;
  }

  async function submitPlaybook(title: string | null, notes: string | null): Promise<string> {
    const validationError = validateSteps(editableSteps.value);
    if (validationError) throw new Error(validationError);
    const piiError = checkForPII(editableSteps.value);
    if (piiError) throw new Error(piiError);

    recordingStatus.value = "submitting";

    const submission: PlaybookSubmission = {
      broker_id: recordingBrokerId.value!,
      broker_name: recordingBrokerName.value!,
      title,
      notes,
      steps: editableSteps.value,
    };

    try {
      const result = await invoke<{ id: string; status: string; message: string }>(
        "submit_playbook",
        { submission }
      );

      // Track the submission locally
      const tracked: TrackedSubmission = {
        playbook_id: result.id,
        broker_id: recordingBrokerId.value!,
        broker_name: recordingBrokerName.value!,
        status: result.status,
        submitted_at: new Date().toISOString(),
      };
      await invoke("track_submission", { submission: tracked });
      trackedSubmissions.value = [...trackedSubmissions.value, tracked];

      resetRecording();
      return result.message;
    } catch (e) {
      recordingStatus.value = "reviewing";
      throw e;
    }
  }

  function resetRecording() {
    if (pollingInterval.value) {
      clearInterval(pollingInterval.value);
      pollingInterval.value = null;
    }
    recordingStatus.value = "idle";
    recordingBrokerId.value = null;
    recordingBrokerName.value = null;
    recordedActions.value = [];
    editableSteps.value = [];
    playbookTitle.value = null;
    editingLocalId.value = null;
    seenActionCount.value = 0;
  }

  // --- Playbook browsing ---

  async function fetchPlaybooks(brokerId: string) {
    loadingPlaybooks.value = true;
    try {
      const list = await invoke<PlaybookSummary[]>("fetch_playbooks", { brokerId });
      playbookCache.value.set(brokerId, list);
    } catch {
      // Network errors are non-fatal for browsing
    } finally {
      loadingPlaybooks.value = false;
    }
  }

  async function fetchPlaybookDetail(id: string) {
    const detail = await invoke<Playbook>("fetch_playbook_detail", { id });
    selectedPlaybook.value = detail;
  }

  async function voteOnPlaybook(id: string, vote: "up" | "down") {
    await invoke("vote_on_playbook", { id, vote });
  }

  function getPlaybooksForBroker(brokerId: string): PlaybookSummary[] {
    return playbookCache.value.get(brokerId) ?? [];
  }

  // --- Local playbooks ---

  async function loadLocalPlaybooks() {
    try {
      localPlaybooks.value = await invoke<LocalPlaybook[]>("get_local_playbooks");
    } catch {
      // Non-fatal
    }
  }

  async function saveLocally(title: string | null, notes: string | null): Promise<void> {
    const validationError = validateSteps(editableSteps.value);
    if (validationError) throw new Error(validationError);
    const now = new Date().toISOString();
    const playbook: LocalPlaybook = {
      id: crypto.randomUUID(),
      brokerId: recordingBrokerId.value!,
      brokerName: recordingBrokerName.value!,
      title,
      notes,
      steps: editableSteps.value,
      createdAt: now,
      updatedAt: now,
    };
    await invoke("save_local_playbook", { playbook });
    localPlaybooks.value = [...localPlaybooks.value, playbook];
    resetRecording();
  }

  async function updateLocalPlaybook(id: string, title: string | null, notes: string | null): Promise<void> {
    const validationError = validateSteps(editableSteps.value);
    if (validationError) throw new Error(validationError);
    const existing = localPlaybooks.value.find((p) => p.id === id);
    if (!existing) throw new Error("Playbook not found");
    const updated: LocalPlaybook = {
      ...existing,
      title,
      notes,
      steps: editableSteps.value,
      updatedAt: new Date().toISOString(),
    };
    await invoke("save_local_playbook", { playbook: updated });
    localPlaybooks.value = localPlaybooks.value.map((p) => (p.id === id ? updated : p));
    resetRecording();
  }

  async function deleteLocalPlaybook(id: string): Promise<void> {
    await invoke("delete_local_playbook", { id });
    localPlaybooks.value = localPlaybooks.value.filter((p) => p.id !== id);
  }

  function loadDraftForEditing(id: string) {
    const playbook = localPlaybooks.value.find((p) => p.id === id);
    if (!playbook) return;
    recordingBrokerId.value = playbook.brokerId;
    recordingBrokerName.value = playbook.brokerName;
    editableSteps.value = playbook.steps.map((s) => ({ ...s }));
    playbookTitle.value = playbook.title ?? null;
    editingLocalId.value = id;
    recordingStatus.value = "reviewing";
  }

  function getLocalPlaybooksForBroker(brokerId: string): LocalPlaybook[] {
    return localPlaybooks.value.filter((p) => p.brokerId === brokerId);
  }

  // --- Submission tracker ---

  async function loadTrackedSubmissions() {
    try {
      trackedSubmissions.value = await invoke<TrackedSubmission[]>("get_tracked_submissions");
    } catch {
      // Non-fatal
    }
  }

  async function refreshSubmissionStatuses() {
    try {
      trackedSubmissions.value = await invoke<TrackedSubmission[]>("refresh_submission_statuses");
    } catch {
      // Non-fatal — API may be unreachable
    }
  }

  return {
    // Recording state
    recordingStatus,
    recordingBrokerId,
    recordingBrokerName,
    recordedActions,
    editableSteps,
    playbookTitle,
    isRecording,
    isReviewing,
    // Playbook browsing
    playbookCache,
    selectedPlaybook,
    loadingPlaybooks,
    // Recording actions
    startRecording,
    stopRecording,
    markCaptcha,
    markUserPrompt,
    reorderStep,
    deleteStep,
    updateStep,
    addUserPromptStep,
    submitPlaybook,
    resetRecording,
    // Browsing actions
    fetchPlaybooks,
    fetchPlaybookDetail,
    voteOnPlaybook,
    getPlaybooksForBroker,
    // Validation
    validateSteps,
    // Local playbook state + actions
    localPlaybooks,
    editingLocalId,
    loadLocalPlaybooks,
    // Submission tracker
    trackedSubmissions,
    hasSubmitted,
    hasApproved,
    loadTrackedSubmissions,
    refreshSubmissionStatuses,
    saveLocally,
    updateLocalPlaybook,
    deleteLocalPlaybook,
    loadDraftForEditing,
    getLocalPlaybooksForBroker,
  };
});
