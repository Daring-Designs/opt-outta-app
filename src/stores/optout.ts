import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "vue-sonner";
import type {
  RunStatus,
  OptOutProgress,
  OptOutComplete,
  UserActionRequired,
} from "../types";

export interface BrokerOutcome {
  brokerId: string;
  brokerName: string;
  succeeded: boolean;
  lastStep: string;
  error: string | null;
}

export const useOptOutStore = defineStore("optout", () => {
  const runId = ref<string | null>(null);
  const status = ref<RunStatus>("idle");
  const currentBrokerId = ref<string | null>(null);
  const currentBrokerName = ref<string | null>(null);
  const currentStep = ref("");
  const brokersCompleted = ref(0);
  const brokersTotal = ref(0);
  const actionRequired = ref<UserActionRequired | null>(null);
  const error = ref<string | null>(null);
  const chromeInstalled = ref<boolean | null>(null);
  const lastResult = ref<OptOutComplete | null>(null);

  // Per-broker tracking during a run
  const brokerProgress = ref<Map<string, { name: string; step: string; error: string | null }>>(new Map());
  const brokerOutcomes = ref<BrokerOutcome[]>([]);

  const isActive = computed(
    () => status.value === "running" || status.value === "waiting_for_user"
  );

  const progressPercent = computed(() => {
    if (brokersTotal.value === 0) return 0;
    return Math.round((brokersCompleted.value / brokersTotal.value) * 100);
  });

  let listenersSetup = false;
  let prevCompleted = 0;

  function finalizeBroker(brokerId: string) {
    const entry = brokerProgress.value.get(brokerId);
    if (!entry) return;
    const succeeded = entry.step === "Opt-out submitted" && !entry.error;
    brokerOutcomes.value.push({
      brokerId,
      brokerName: entry.name,
      succeeded,
      lastStep: entry.step,
      error: entry.error,
    });
  }

  async function setupListeners() {
    if (listenersSetup) return;
    listenersSetup = true;

    await listen<OptOutProgress>("opt-out-progress", (event) => {
      const p = event.payload;
      runId.value = p.run_id;
      status.value = p.status;
      currentBrokerId.value = p.broker_id;
      currentBrokerName.value = p.broker_name;
      currentStep.value = p.current_step;
      brokersTotal.value = p.brokers_total;
      actionRequired.value = p.action_required;
      error.value = p.error;

      // Show toast for broker-level errors
      if (p.error) {
        toast.error(p.broker_name, { description: p.error });
      }

      // Track per-broker state
      brokerProgress.value.set(p.broker_id, {
        name: p.broker_name,
        step: p.current_step,
        error: p.error,
      });

      // When brokers_completed increments, finalize the previous broker
      if (p.brokers_completed > prevCompleted && currentBrokerId.value) {
        finalizeBroker(p.broker_id);
      }
      prevCompleted = p.brokers_completed;
      brokersCompleted.value = p.brokers_completed;
    });

    await listen<OptOutComplete>("opt-out-complete", (event) => {
      lastResult.value = event.payload;
      status.value = "completed";
      actionRequired.value = null;

      // Finalize any remaining broker that didn't get a completed event
      if (brokerOutcomes.value.length < brokerProgress.value.size) {
        for (const [id] of brokerProgress.value) {
          if (!brokerOutcomes.value.find((o) => o.brokerId === id)) {
            finalizeBroker(id);
          }
        }
      }

      // Show summary toast
      const r = event.payload;
      if (r.failed > 0 && r.succeeded === 0) {
        toast.error("Run failed", { description: `${r.failed} of ${r.total} brokers failed` });
      } else if (r.failed > 0) {
        toast.warning("Run completed with errors", { description: `${r.succeeded} succeeded, ${r.failed} failed` });
      } else {
        toast.success("Run complete", { description: `${r.succeeded} of ${r.total} brokers succeeded` });
      }

      // Reload history so dashboard/brokers views reflect new data
      import("./history").then(({ useHistoryStore }) => {
        useHistoryStore().loadHistory();
      });
    });
  }

  async function checkChromeInstalled(): Promise<boolean> {
    const installed = await invoke<boolean>("check_chrome_installed");
    chromeInstalled.value = installed;
    return installed;
  }

  async function startRun(
    brokerIds: string[],
    playbookSelections?: Record<string, string>
  ) {
    await setupListeners();
    error.value = null;
    lastResult.value = null;
    brokerProgress.value = new Map();
    brokerOutcomes.value = [];
    prevCompleted = 0;
    try {
      const id = await invoke<string>("start_opt_out_run", {
        brokerIds,
        playbookSelections: playbookSelections ?? null,
      });
      runId.value = id;
      status.value = "running";
      brokersCompleted.value = 0;
      brokersTotal.value = brokerIds.length;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error("Failed to start opt-out run", { description: msg });
      throw e;
    }
  }

  async function continueAfterUserAction(response?: string) {
    await invoke("continue_opt_out", { response: response ?? null });
    actionRequired.value = null;
    status.value = "running";
  }

  async function retryFailedStep() {
    await continueAfterUserAction("retry");
  }

  async function skipFailedStep() {
    await continueAfterUserAction("skip");
  }

  async function abortBroker() {
    await continueAfterUserAction("abort");
  }

  async function cancelRun() {
    await invoke("cancel_opt_out");
    status.value = "failed";
    actionRequired.value = null;
  }

  function generateReport(): string {
    const result = lastResult.value;
    const date = new Date().toLocaleString();
    const lines: string[] = [
      "Opt-Outta Run Report",
      `Date: ${date}`,
      `Run ID: ${runId.value}`,
      "",
      `Results: ${result?.succeeded ?? 0} succeeded, ${result?.failed ?? 0} failed out of ${result?.total ?? 0} brokers`,
      "",
      "Details:",
      "─".repeat(50),
    ];

    for (const outcome of brokerOutcomes.value) {
      const icon = outcome.succeeded ? "[OK]" : "[FAIL]";
      lines.push(`${icon}  ${outcome.brokerName}`);
      if (outcome.error) {
        lines.push(`       Error: ${outcome.error}`);
      }
    }

    lines.push("─".repeat(50));
    lines.push("");
    lines.push("Generated by Opt-Outta");
    return lines.join("\n");
  }

  async function copyReport(): Promise<boolean> {
    try {
      await navigator.clipboard.writeText(generateReport());
      return true;
    } catch {
      return false;
    }
  }

  function reset() {
    status.value = "idle";
    runId.value = null;
    currentBrokerId.value = null;
    currentBrokerName.value = null;
    currentStep.value = "";
    brokersCompleted.value = 0;
    brokersTotal.value = 0;
    actionRequired.value = null;
    error.value = null;
    brokerOutcomes.value = [];
    brokerProgress.value = new Map();
    prevCompleted = 0;
  }

  return {
    runId,
    status,
    currentBrokerId,
    currentBrokerName,
    currentStep,
    brokersCompleted,
    brokersTotal,
    actionRequired,
    error,
    chromeInstalled,
    lastResult,
    brokerOutcomes,
    isActive,
    progressPercent,
    setupListeners,
    checkChromeInstalled,
    startRun,
    continueAfterUserAction,
    retryFailedStep,
    skipFailedStep,
    abortBroker,
    cancelRun,
    generateReport,
    copyReport,
    reset,
  };
});
