import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { SubmissionRecord, BrokerStatus } from "../types";

export const useHistoryStore = defineStore("history", () => {
  const submissions = ref<SubmissionRecord[]>([]);
  const latestPerBroker = ref<SubmissionRecord[]>([]);
  const relistAlerts = ref<SubmissionRecord[]>([]);
  const loading = ref(false);

  const confirmedCount = computed(
    () => latestPerBroker.value.filter((r) => r.status === "confirmed").length
  );

  const pendingCount = computed(
    () =>
      latestPerBroker.value.filter(
        (r) => r.status === "submitted" || r.status === "pending_verification"
      ).length
  );

  const failedCount = computed(
    () => latestPerBroker.value.filter((r) => r.status === "failed").length
  );

  const relistCount = computed(() => relistAlerts.value.length);

  async function loadHistory() {
    loading.value = true;
    try {
      const [allSubs, latest, alerts] = await Promise.all([
        invoke<SubmissionRecord[]>("get_submissions"),
        invoke<SubmissionRecord[]>("get_latest_submissions"),
        invoke<SubmissionRecord[]>("get_relisting_alerts"),
      ]);
      submissions.value = allSubs;
      latestPerBroker.value = latest;
      relistAlerts.value = alerts;
    } catch (e) {
      console.error("Failed to load history:", e);
    } finally {
      loading.value = false;
    }
  }

  function getStatusForBroker(brokerId: string): BrokerStatus {
    const record = latestPerBroker.value.find(
      (r) => r.broker_id === brokerId
    );
    if (!record) return "not_started" as BrokerStatus;

    // Map backend status strings to BrokerStatus enum values
    switch (record.status) {
      case "submitted":
        return "submitted" as BrokerStatus;
      case "pending_verification":
        return "pending_verification" as BrokerStatus;
      case "confirmed":
        return "confirmed" as BrokerStatus;
      case "failed":
        return "failed" as BrokerStatus;
      case "re_listed":
        return "re_listed" as BrokerStatus;
      default:
        return "not_started" as BrokerStatus;
    }
  }

  async function confirmSubmission(id: string) {
    await invoke("update_submission_status", { id, status: "confirmed" });
    await loadHistory();
  }

  return {
    submissions,
    latestPerBroker,
    relistAlerts,
    loading,
    confirmedCount,
    pendingCount,
    failedCount,
    relistCount,
    loadHistory,
    getStatusForBroker,
    confirmSubmission,
  };
});
