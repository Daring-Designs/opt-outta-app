<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useBrokersStore } from "../stores/brokers";
import { useHistoryStore } from "../stores/history";
import { useOptOutStore } from "../stores/optout";
import { usePlaybooksStore } from "../stores/playbooks";
import type { Broker, BrokerStatus } from "../types";
import StatusBadge from "../components/StatusBadge.vue";
import OptOutRunner from "../components/OptOutRunner.vue";
import UserActionModal from "../components/UserActionModal.vue";
import RunConfirmModal from "../components/RunConfirmModal.vue";
import PlaybookRecorder from "../components/PlaybookRecorder.vue";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ChevronRight, ChevronUp, ChevronDown } from "lucide-vue-next";

const brokersStore = useBrokersStore();
const historyStore = useHistoryStore();
const optOutStore = useOptOutStore();
const playbooksStore = usePlaybooksStore();

const expandedBrokerId = ref<string | null>(null);
const loadingBrokerId = ref<string | null>(null);
const runError = ref<string | null>(null);

const deletingLocalId = ref<string | null>(null);

// --- Per-broker run ---
const runBrokerId = ref<string | null>(null);

const runBroker = computed<Broker | null>(() => {
  if (!runBrokerId.value) return null;
  return brokersStore.brokers.find((b) => b.id === runBrokerId.value) ?? null;
});

onMounted(async () => {
  if (brokersStore.brokers.length === 0) await brokersStore.loadBrokers();
  await historyStore.loadHistory();
  await playbooksStore.loadLocalPlaybooks();
});

// --- Per-broker run ---

async function openRunForBroker(broker: Broker) {
  runError.value = null;
  const hasChrome = await optOutStore.checkChromeInstalled();
  if (!hasChrome) {
    runError.value =
      "Google Chrome is not installed. Please install it to continue.";
    return;
  }
  runBrokerId.value = broker.id;
}

async function handleConfirmRun(playbookSelections: Record<string, string>) {
  if (!runBrokerId.value) return;
  const brokerId = runBrokerId.value;
  runBrokerId.value = null;
  runError.value = null;
  try {
    await optOutStore.startRun(
      [brokerId],
      Object.keys(playbookSelections).length > 0
        ? playbookSelections
        : undefined
    );
  } catch (e) {
    runError.value = String(e);
  }
}

function brokerHasPlaybooks(brokerId: string): boolean {
  return (
    playbooksStore.getPlaybooksForBroker(brokerId).length > 0 ||
    playbooksStore.getLocalPlaybooksForBroker(brokerId).length > 0
  );
}

// --- Playbook expand ---

async function toggleExpand(brokerId: string) {
  if (expandedBrokerId.value === brokerId) {
    expandedBrokerId.value = null;
    return;
  }
  expandedBrokerId.value = brokerId;
  if (!playbooksStore.playbookCache.has(brokerId)) {
    loadingBrokerId.value = brokerId;
    await playbooksStore.fetchPlaybooks(brokerId);
    loadingBrokerId.value = null;
  }
}

async function startRecording(
  brokerId: string,
  brokerName: string,
  optOutUrl: string
) {
  await playbooksStore.startRecording(brokerId, brokerName, optOutUrl);
}

async function handleVote(id: string, vote: "up" | "down") {
  await playbooksStore.voteOnPlaybook(id, vote);
}

// --- Local playbook actions ---

function editLocalPlaybook(id: string) {
  playbooksStore.loadDraftForEditing(id);
}

async function deleteLocalPlaybook(id: string) {
  deletingLocalId.value = id;
  try {
    await playbooksStore.deleteLocalPlaybook(id);
  } finally {
    deletingLocalId.value = null;
  }
}

function submitLocalPlaybook(id: string) {
  playbooksStore.loadDraftForEditing(id);
  // Clear editingLocalId so the recorder treats it as a new community submission
  playbooksStore.editingLocalId = null;
}

// --- Helpers ---

function difficultyColor(difficulty: string): string {
  switch (difficulty) {
    case "easy":
      return "text-green-600";
    case "medium":
      return "text-yellow-600";
    case "hard":
      return "text-red-600";
    default:
      return "text-muted-foreground";
  }
}
</script>

<template>
  <div class="mx-auto max-w-5xl">
    <!-- Header -->
    <div class="mb-6">
      <h1 class="text-2xl font-bold">Brokers</h1>
      <p class="mt-1 text-sm text-muted-foreground">
        View community playbooks, record your own, or run opt-outs.
      </p>
    </div>

    <!-- Filters -->
    <div class="mb-4 flex gap-3">
      <Input
        v-model="brokersStore.searchQuery"
        type="text"
        placeholder="Search brokers..."
        class="flex-1"
      />
      <select
        v-model="brokersStore.categoryFilter"
        class="rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
      >
        <option :value="null">All Categories</option>
        <option
          v-for="cat in brokersStore.categories"
          :key="cat"
          :value="cat"
        >
          {{ cat }}
        </option>
      </select>
    </div>

    <!-- Loading -->
    <div
      v-if="brokersStore.loading"
      class="py-12 text-center text-muted-foreground"
    >
      Loading brokers...
    </div>

    <!-- Broker list -->
    <div v-else class="space-y-2">
      <div
        v-for="broker in brokersStore.filteredBrokers"
        :key="broker.id"
        class="overflow-hidden rounded-lg border border-border bg-card"
      >
        <!-- Main broker row -->
        <div class="flex items-center gap-3 px-4 py-3 hover:bg-accent/50">
          <!-- Expand toggle + broker info -->
          <div
            class="flex flex-1 cursor-pointer items-center justify-between"
            @click="toggleExpand(broker.id)"
          >
            <div class="flex items-center gap-3">
              <ChevronRight
                class="h-4 w-4 flex-shrink-0 text-muted-foreground transition-transform"
                :class="{ 'rotate-90': expandedBrokerId === broker.id }"
              />
              <div>
                <div class="text-sm font-medium">
                  {{ broker.name }}
                </div>
                <div class="text-xs text-muted-foreground">{{ broker.url }}</div>
              </div>
            </div>

            <div class="flex items-center gap-4">
              <!-- Category -->
              <span class="hidden text-xs text-muted-foreground sm:inline">{{
                broker.category
              }}</span>

              <!-- Difficulty -->
              <span
                class="text-xs font-medium"
                :class="difficultyColor(broker.difficulty)"
              >
                {{ broker.difficulty }}
              </span>

              <!-- Playbook badge -->
              <span
                v-if="
                  playbooksStore.getPlaybooksForBroker(broker.id).length > 0
                "
                class="rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700 dark:bg-green-900/30 dark:text-green-400"
              >
                {{
                  playbooksStore.getPlaybooksForBroker(broker.id).length
                }}
                playbook{{
                  playbooksStore.getPlaybooksForBroker(broker.id).length > 1
                    ? "s"
                    : ""
                }}
              </span>

              <!-- Status -->
              <StatusBadge
                :status="
                  historyStore.getStatusForBroker(broker.id) as BrokerStatus
                "
              />
            </div>
          </div>

          <!-- Run button -->
          <Button
            v-if="brokerHasPlaybooks(broker.id)"
            size="sm"
            :disabled="optOutStore.isActive"
            @click.stop="openRunForBroker(broker)"
          >
            Run
          </Button>
        </div>

        <!-- Expanded: playbook detail panel -->
        <div
          v-if="expandedBrokerId === broker.id"
          class="border-t border-border bg-muted/50 px-4 py-3"
        >
          <!-- Local playbooks section -->
          <div
            v-if="playbooksStore.getLocalPlaybooksForBroker(broker.id).length > 0"
            class="mb-4"
          >
            <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-green-600">
              Local Drafts
            </h3>
            <div class="space-y-2">
              <div
                v-for="lp in playbooksStore.getLocalPlaybooksForBroker(broker.id)"
                :key="lp.id"
                class="rounded-lg border border-green-200 bg-card p-3 dark:border-green-800"
              >
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <span class="text-sm font-medium">{{
                      lp.title || "Untitled"
                    }}</span>
                    <span class="text-xs text-muted-foreground"
                      >{{ lp.steps.length }} steps</span
                    >
                    <span class="text-xs text-muted-foreground">{{
                      lp.updatedAt.slice(0, 10)
                    }}</span>
                  </div>
                  <div class="flex items-center gap-1">
                    <Button variant="ghost" size="sm" class="h-7 text-xs" @click.stop="editLocalPlaybook(lp.id)">
                      Edit
                    </Button>
                    <Button variant="ghost" size="sm" class="h-7 text-xs text-green-600" @click.stop="submitLocalPlaybook(lp.id)">
                      Submit
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 text-xs text-destructive"
                      :disabled="deletingLocalId === lp.id"
                      @click.stop="deleteLocalPlaybook(lp.id)"
                    >
                      {{ deletingLocalId === lp.id ? "..." : "Delete" }}
                    </Button>
                  </div>
                </div>
                <p v-if="lp.notes" class="mt-1 text-xs text-muted-foreground">
                  {{ lp.notes }}
                </p>
              </div>
            </div>
          </div>

          <div class="mb-3 flex items-center justify-between">
            <h3 class="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Community Playbooks
            </h3>
            <Button
              variant="outline"
              size="sm"
              :disabled="playbooksStore.isRecording"
              @click.stop="
                startRecording(broker.id, broker.name, broker.opt_out_url)
              "
            >
              Record Playbook
            </Button>
          </div>

          <div
            v-if="loadingBrokerId === broker.id"
            class="py-4 text-center text-sm text-muted-foreground"
          >
            Loading playbooks...
          </div>

          <div
            v-else-if="
              playbooksStore.getPlaybooksForBroker(broker.id).length === 0
            "
            class="py-4 text-center text-sm text-muted-foreground"
          >
            No community playbooks yet. Be the first to record one!
          </div>

          <div v-else class="space-y-2">
            <div
              v-for="pb in playbooksStore.getPlaybooksForBroker(broker.id)"
              :key="pb.id"
              class="rounded-lg border border-border bg-card p-3"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                  <span class="text-sm font-medium"
                    >v{{ pb.version }}</span
                  >
                  <span class="text-xs text-muted-foreground"
                    >{{ pb.steps_count }} steps</span
                  >
                  <span class="text-xs text-muted-foreground">{{
                    pb.created_at.slice(0, 10)
                  }}</span>
                </div>
                <div class="flex items-center gap-3">
                  <!-- Votes -->
                  <div class="flex items-center gap-1">
                    <button
                      class="rounded p-1 text-muted-foreground hover:text-green-600"
                      @click="handleVote(pb.id, 'up')"
                    >
                      <ChevronUp class="h-4 w-4" />
                    </button>
                    <span
                      class="text-xs font-medium"
                      :class="
                        pb.upvotes - pb.downvotes >= 0
                          ? 'text-green-600'
                          : 'text-red-600'
                      "
                    >
                      {{ pb.upvotes - pb.downvotes }}
                    </span>
                    <button
                      class="rounded p-1 text-muted-foreground hover:text-red-600"
                      @click="handleVote(pb.id, 'down')"
                    >
                      <ChevronDown class="h-4 w-4" />
                    </button>
                  </div>

                  <!-- Success rate -->
                  <span class="text-xs text-muted-foreground">
                    <span class="text-green-600">{{
                      pb.success_count
                    }}</span>
                    /
                    <span class="text-red-500">{{
                      pb.failure_count
                    }}</span>
                  </span>
                </div>
              </div>
              <p v-if="pb.notes" class="mt-1 text-xs text-muted-foreground">
                {{ pb.notes }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="
        !brokersStore.loading && brokersStore.filteredBrokers.length === 0
      "
      class="py-12 text-center text-sm text-muted-foreground"
    >
      No brokers match your search.
    </div>

    <p class="mt-4 text-xs text-muted-foreground">
      Registry version: {{ brokersStore.version }} &middot;
      {{ brokersStore.brokers.length }} brokers
    </p>

    <!-- Error -->
    <p v-if="runError" class="mt-3 text-sm text-destructive">{{ runError }}</p>

    <!-- Per-broker run confirmation modal -->
    <RunConfirmModal
      v-if="runBrokerId && runBroker"
      :brokers="[runBroker]"
      @confirm="handleConfirmRun"
      @cancel="runBrokerId = null"
    />

    <!-- Recording overlay -->
    <PlaybookRecorder />

    <!-- Opt-out modals -->
    <UserActionModal />
    <OptOutRunner />
  </div>
</template>
