<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { usePlaybooksStore } from "../stores/playbooks";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronUp, ChevronDown } from "lucide-vue-next";
import type { PlaybookReportEntry } from "../types";

const route = useRoute();
const router = useRouter();
const playbooksStore = usePlaybooksStore();

const loading = ref(true);
const error = ref<string | null>(null);
const reports = ref<PlaybookReportEntry[]>([]);
const reportsLoading = ref(false);

const playbook = computed(() => playbooksStore.selectedPlaybook);

onMounted(async () => {
  const id = route.params.id as string;
  try {
    await playbooksStore.fetchPlaybookDetail(id);
    // Fetch reports in parallel (fire-and-forget style for loading state)
    reportsLoading.value = true;
    invoke<PlaybookReportEntry[]>("fetch_playbook_reports", { id })
      .then((r) => { reports.value = r; })
      .catch(() => { /* silently ignore report fetch failures */ })
      .finally(() => { reportsLoading.value = false; });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});

function goBack() {
  router.back();
}

function actionColor(action: string): string {
  switch (action) {
    case "navigate":
      return "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400";
    case "click":
    case "find_and_click":
      return "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400";
    case "fill":
    case "select":
    case "check":
      return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400";
    case "captcha":
      return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "user_prompt":
      return "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400";
    case "wait":
    case "wait_for":
      return "bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400";
    default:
      return "bg-gray-100 text-gray-700 dark:bg-gray-900/30 dark:text-gray-400";
  }
}

async function handleVote(vote: "up" | "down") {
  if (!playbook.value) return;
  await playbooksStore.voteOnPlaybook(playbook.value.id, vote);
}
</script>

<template>
  <div class="mx-auto max-w-3xl">
    <!-- Back button -->
    <button
      class="mb-4 flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
      @click="goBack"
    >
      <ChevronLeft class="h-4 w-4" />
      Back to Brokers
    </button>

    <!-- Loading -->
    <div v-if="loading" class="py-12 text-center text-muted-foreground">
      Loading playbook...
    </div>

    <!-- Error -->
    <div v-else-if="error" class="py-12 text-center">
      <p class="text-sm text-destructive">{{ error }}</p>
      <Button variant="outline" size="sm" class="mt-3" @click="goBack">
        Go Back
      </Button>
    </div>

    <!-- Detail -->
    <template v-else-if="playbook">
      <!-- Header -->
      <div class="mb-6">
        <h1 class="text-2xl font-bold">
          {{ playbook.title || `${playbook.broker_name} Playbook` }}
        </h1>
        <p class="mt-1 text-sm text-muted-foreground">
          {{ playbook.broker_name }} &middot; v{{ playbook.version }} &middot; {{ playbook.created_at.slice(0, 10) }}
        </p>
      </div>

      <!-- Stats row -->
      <div class="mb-6 flex flex-wrap items-center gap-4">
        <!-- Votes -->
        <div class="flex items-center gap-1">
          <button
            class="rounded p-1"
            :class="playbooksStore.getUserVote(playbook.id) === 'up' ? 'text-green-600' : 'text-muted-foreground hover:text-green-600'"
            @click="handleVote('up')"
          >
            <ChevronUp class="h-4 w-4" />
          </button>
          <span
            class="text-sm font-medium"
            :class="playbook.upvotes - playbook.downvotes >= 0 ? 'text-green-600' : 'text-red-600'"
          >
            {{ playbook.upvotes - playbook.downvotes }}
          </span>
          <button
            class="rounded p-1"
            :class="playbooksStore.getUserVote(playbook.id) === 'down' ? 'text-red-600' : 'text-muted-foreground hover:text-red-600'"
            @click="handleVote('down')"
          >
            <ChevronDown class="h-4 w-4" />
          </button>
        </div>

        <span class="text-xs text-muted-foreground">|</span>

        <!-- Success / failure -->
        <span class="text-sm text-muted-foreground">
          <span class="text-green-600">{{ playbook.success_count }}</span> succeeded &middot;
          <span class="text-red-500">{{ playbook.failure_count }}</span> failed
        </span>

        <span class="text-xs text-muted-foreground">|</span>

        <!-- Status -->
        <span
          class="rounded-full px-2 py-0.5 text-xs font-medium"
          :class="playbook.status === 'approved' ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400'"
        >
          {{ playbook.status }}
        </span>

        <!-- Steps count -->
        <span class="text-sm text-muted-foreground">
          {{ playbook.steps.length }} steps
        </span>
      </div>

      <!-- Notes -->
      <div v-if="playbook.notes" class="mb-6 rounded-lg border border-border bg-card p-4">
        <h2 class="mb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">Notes</h2>
        <p class="text-sm">{{ playbook.notes }}</p>
      </div>

      <!-- Steps -->
      <div>
        <h2 class="mb-3 text-xs font-medium uppercase tracking-wide text-muted-foreground">Steps</h2>
        <div class="space-y-2">
          <div
            v-for="step in playbook.steps"
            :key="step.position"
            class="rounded-lg border border-border bg-card p-3"
          >
            <div class="flex items-start gap-3">
              <!-- Step number -->
              <span class="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium">
                {{ step.position }}
              </span>

              <div class="flex-1 min-w-0">
                <!-- Action badge + description -->
                <div class="flex items-center gap-2">
                  <span
                    class="rounded px-1.5 py-0.5 text-xs font-medium"
                    :class="actionColor(step.action)"
                  >
                    {{ step.action }}
                  </span>
                  <span class="text-sm">{{ step.description }}</span>
                  <span
                    v-if="step.optional"
                    class="rounded-full bg-muted px-1.5 py-0.5 text-xs text-muted-foreground"
                  >
                    optional
                  </span>
                </div>

                <!-- Details -->
                <div class="mt-1.5 space-y-1">
                  <p v-if="step.selector" class="font-mono text-xs text-muted-foreground truncate">
                    {{ step.selector }}
                  </p>
                  <p v-if="step.profile_key" class="text-xs text-muted-foreground">
                    Profile field: <span class="font-medium text-foreground">{{ step.profile_key }}</span>
                  </p>
                  <p v-if="step.value && step.action === 'navigate'" class="text-xs text-blue-600 dark:text-blue-400 truncate">
                    {{ step.value }}
                  </p>
                  <p v-else-if="step.value" class="text-xs text-muted-foreground">
                    Value: {{ step.value }}
                  </p>
                  <p v-if="step.instructions" class="text-xs text-muted-foreground italic">
                    {{ step.instructions }}
                  </p>
                  <p v-if="step.wait_after_ms > 0" class="text-xs text-muted-foreground">
                    Wait: {{ step.wait_after_ms }}ms
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Community Reports -->
      <div class="mt-8">
        <h2 class="mb-3 text-xs font-medium uppercase tracking-wide text-muted-foreground">Community Reports</h2>

        <div v-if="reportsLoading" class="text-sm text-muted-foreground">
          Loading reports...
        </div>

        <div v-else-if="reports.length === 0" class="text-sm text-muted-foreground">
          No community reports yet.
        </div>

        <div v-else class="space-y-2">
          <div
            v-for="(report, idx) in reports"
            :key="idx"
            class="rounded-lg border border-border bg-card p-3"
          >
            <div class="flex items-center gap-3">
              <!-- Outcome badge -->
              <span
                class="rounded-full px-2 py-0.5 text-xs font-medium"
                :class="report.outcome === 'success'
                  ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                  : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'"
              >
                {{ report.outcome }}
              </span>

              <!-- Date -->
              <span class="text-xs text-muted-foreground">
                {{ report.created_at.slice(0, 10) }}
              </span>

              <!-- App version -->
              <span class="font-mono text-xs text-muted-foreground">
                v{{ report.app_version }}
              </span>
            </div>

            <!-- Failure details -->
            <div v-if="report.outcome !== 'success'" class="mt-1.5 space-y-0.5">
              <p v-if="report.failure_step != null" class="text-xs text-muted-foreground">
                Failed at step <span class="font-medium text-foreground">{{ report.failure_step }}</span>
              </p>
              <p v-if="report.error_message" class="text-xs text-destructive">
                {{ report.error_message }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>
