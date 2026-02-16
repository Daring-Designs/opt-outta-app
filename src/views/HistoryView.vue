<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useHistoryStore } from "../stores/history";
import { useBrokersStore } from "../stores/brokers";
import type { BrokerStatus } from "../types";
import StatusBadge from "../components/StatusBadge.vue";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

const historyStore = useHistoryStore();
const brokersStore = useBrokersStore();

const statusFilter = ref<string | null>(null);

onMounted(async () => {
  await historyStore.loadHistory();
  if (brokersStore.brokers.length === 0) await brokersStore.loadBrokers();
});

function brokerName(brokerId: string): string {
  const broker = brokersStore.brokers.find((b) => b.id === brokerId);
  return broker?.name || brokerId;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "\u2014";
  return new Date(dateStr).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function statusToBrokerStatus(status: string): BrokerStatus {
  const map: Record<string, BrokerStatus> = {
    submitted: "submitted" as BrokerStatus,
    pending_verification: "pending_verification" as BrokerStatus,
    confirmed: "confirmed" as BrokerStatus,
    failed: "failed" as BrokerStatus,
    re_listed: "re_listed" as BrokerStatus,
  };
  return map[status] || ("not_started" as BrokerStatus);
}

const filteredSubmissions = computed(() => {
  if (!statusFilter.value) return historyStore.submissions;
  return historyStore.submissions.filter(
    (r) => r.status === statusFilter.value
  );
});

const sortedSubmissions = computed(() => {
  return [...filteredSubmissions.value].sort(
    (a, b) => new Date(b.submitted_at).getTime() - new Date(a.submitted_at).getTime()
  );
});
</script>

<template>
  <div class="mx-auto max-w-4xl">
    <h1 class="mb-6 text-2xl font-bold">History</h1>

    <!-- Empty state -->
    <Card
      v-if="!historyStore.loading && historyStore.submissions.length === 0"
      class="py-16 text-center"
    >
      <p class="text-lg font-medium text-muted-foreground">No opt-out history yet</p>
      <p class="mt-2 text-sm text-muted-foreground">
        Run your first opt-out to see results here.
      </p>
    </Card>

    <!-- History table -->
    <template v-else>
      <!-- Filter -->
      <div class="mb-4">
        <select
          v-model="statusFilter"
          class="rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
        >
          <option :value="null">All Statuses</option>
          <option value="submitted">Submitted</option>
          <option value="pending_verification">Pending Verification</option>
          <option value="confirmed">Confirmed</option>
          <option value="failed">Failed</option>
          <option value="re_listed">Re-listed</option>
        </select>
      </div>

      <Card class="overflow-hidden">
        <table class="w-full">
          <thead>
            <tr class="border-b border-border bg-muted text-left text-xs font-medium uppercase tracking-wide text-muted-foreground">
              <th class="px-4 py-3">Broker</th>
              <th class="px-4 py-3">Status</th>
              <th class="px-4 py-3">Submitted</th>
              <th class="px-4 py-3">Confirmed</th>
              <th class="px-4 py-3">Next Check</th>
              <th class="px-4 py-3">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            <tr
              v-for="record in sortedSubmissions"
              :key="record.id"
              class="hover:bg-accent/50"
            >
              <td class="px-4 py-3">
                <div class="text-sm font-medium">
                  {{ brokerName(record.broker_id) }}
                </div>
                <div v-if="record.error_message" class="text-xs text-destructive">
                  {{ record.error_message }}
                </div>
              </td>
              <td class="px-4 py-3">
                <StatusBadge :status="statusToBrokerStatus(record.status)" />
              </td>
              <td class="px-4 py-3 text-sm text-muted-foreground">
                {{ formatDate(record.submitted_at) }}
              </td>
              <td class="px-4 py-3 text-sm text-muted-foreground">
                {{ formatDate(record.confirmed_at) }}
              </td>
              <td class="px-4 py-3 text-sm text-muted-foreground">
                {{ formatDate(record.next_check_date) }}
              </td>
              <td class="px-4 py-3">
                <Button
                  v-if="record.status === 'submitted' || record.status === 'pending_verification'"
                  variant="outline"
                  size="sm"
                  class="border-green-300 text-green-700 hover:bg-green-50 dark:border-green-700 dark:text-green-400 dark:hover:bg-green-900/20"
                  @click="historyStore.confirmSubmission(record.id)"
                >
                  Confirm
                </Button>
              </td>
            </tr>
          </tbody>
        </table>
        <div
          v-if="sortedSubmissions.length === 0"
          class="py-8 text-center text-sm text-muted-foreground"
        >
          No records match this filter.
        </div>
      </Card>

      <p class="mt-4 text-xs text-muted-foreground">
        {{ historyStore.submissions.length }} total records
      </p>
    </template>
  </div>
</template>
