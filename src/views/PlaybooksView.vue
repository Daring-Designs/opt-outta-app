<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
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
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { ChevronRight, ChevronUp, ChevronDown, Plus, Play } from "lucide-vue-next";
import { invoke } from "@tauri-apps/api/core";

const router = useRouter();

const brokersStore = useBrokersStore();
const historyStore = useHistoryStore();
const optOutStore = useOptOutStore();
const playbooksStore = usePlaybooksStore();

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

// --- Playbook expand ---

async function toggleExpand(brokerId: string) {
  if (playbooksStore.expandedBrokerId === brokerId) {
    playbooksStore.expandedBrokerId = null;
    return;
  }
  playbooksStore.expandedBrokerId = brokerId;
  if (!(brokerId in playbooksStore.playbookCache)) {
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

const confirmDeleteId = ref<string | null>(null);

async function confirmDeleteLocalPlaybook() {
  if (!confirmDeleteId.value) return;
  deletingLocalId.value = confirmDeleteId.value;
  try {
    await playbooksStore.deleteLocalPlaybook(confirmDeleteId.value);
  } finally {
    deletingLocalId.value = null;
    confirmDeleteId.value = null;
  }
}

function submitLocalPlaybook(id: string) {
  playbooksStore.loadDraftForEditing(id);
  playbooksStore.submittingFromLocalId = id;
  // Clear editingLocalId so the recorder treats it as a new community submission
  playbooksStore.editingLocalId = null;
}

// --- Broker suggestion ---

const showSuggestDialog = ref(false);
const suggestName = ref("");
const suggestUrl = ref("");
const suggestNotes = ref("");
const suggestSubmitting = ref(false);
const suggestError = ref<string | null>(null);
const suggestSuccess = ref(false);

async function handleSuggestBroker() {
  if (!suggestName.value.trim() || !suggestUrl.value.trim()) return;
  suggestSubmitting.value = true;
  suggestError.value = null;
  try {
    await invoke("suggest_broker", {
      name: suggestName.value.trim(),
      url: suggestUrl.value.trim(),
      notes: suggestNotes.value.trim(),
    });
    suggestSuccess.value = true;
    suggestName.value = "";
    suggestUrl.value = "";
    suggestNotes.value = "";
  } catch (e) {
    suggestError.value = String(e);
  } finally {
    suggestSubmitting.value = false;
  }
}

function closeSuggestDialog() {
  showSuggestDialog.value = false;
  suggestSuccess.value = false;
  suggestError.value = null;
}

// --- Submission status for local drafts ---

function getSubmissionStatus(localId: string): string | null {
  const sub = playbooksStore.trackedSubmissions.find(
    (s) => s.local_playbook_id === localId
  );
  return sub?.status ?? null;
}

function isSubmissionPending(localId: string): boolean {
  const status = getSubmissionStatus(localId);
  return !!status && status !== "approved" && status !== "rejected";
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
    <div class="mb-6 flex items-start justify-between">
      <div>
        <h1 class="text-2xl font-bold">Brokers</h1>
        <p class="mt-1 text-sm text-muted-foreground">
          View community playbooks, record your own, or run opt-outs.
        </p>
      </div>
      <Button variant="outline" size="sm" class="gap-1.5" @click="showSuggestDialog = true">
        <Plus class="h-4 w-4" />
        Suggest Broker
      </Button>
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
                :class="{ 'rotate-90': playbooksStore.expandedBrokerId === broker.id }"
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
          <button
            class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-green-500/20 text-green-400 transition-colors hover:bg-green-500/30 disabled:opacity-30"
            :disabled="optOutStore.isActive"
            @click.stop="openRunForBroker(broker)"
          >
            <Play class="h-3.5 w-3.5 fill-current" />
          </button>
        </div>

        <!-- Expanded: playbook detail panel -->
        <div
          v-if="playbooksStore.expandedBrokerId === broker.id"
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
                    <span
                      v-if="isSubmissionPending(lp.id)"
                      class="rounded-full bg-yellow-100 px-2 py-0.5 text-xs font-medium text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400"
                    >
                      Pending Review
                    </span>
                    <span
                      v-else-if="getSubmissionStatus(lp.id) === 'rejected'"
                      class="rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700 dark:bg-red-900/30 dark:text-red-400"
                    >
                      Rejected
                    </span>
                  </div>
                  <div class="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 text-xs"
                      :disabled="isSubmissionPending(lp.id)"
                      @click.stop="editLocalPlaybook(lp.id)"
                    >
                      Edit
                    </Button>
                    <Button
                      v-if="!lp.submittedAt"
                      variant="ghost"
                      size="sm"
                      class="h-7 text-xs text-green-600"
                      @click.stop="submitLocalPlaybook(lp.id)"
                    >
                      Submit
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="h-7 text-xs text-destructive"
                      :disabled="isSubmissionPending(lp.id)"
                      @click.stop="confirmDeleteId = lp.id"
                    >
                      Delete
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
              class="cursor-pointer rounded-lg border border-border bg-card p-3 transition-colors hover:bg-accent/50"
              @click="router.push({ name: 'playbook-detail', params: { id: pb.id } })"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                  <span class="text-sm font-medium">{{
                    pb.title || `${pb.broker_name} Playbook`
                  }}</span>
                  <span class="text-xs text-muted-foreground">v{{ pb.version }}</span>
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
                      class="rounded p-1"
                      :class="playbooksStore.getUserVote(pb.id) === 'up' ? 'text-green-600' : 'text-muted-foreground hover:text-green-600'"
                      @click.stop="handleVote(pb.id, 'up')"
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
                      class="rounded p-1"
                      :class="playbooksStore.getUserVote(pb.id) === 'down' ? 'text-red-600' : 'text-muted-foreground hover:text-red-600'"
                      @click.stop="handleVote(pb.id, 'down')"
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

                  <ChevronRight class="h-4 w-4 text-muted-foreground" />
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

    <!-- Delete confirmation dialog -->
    <Dialog :open="!!confirmDeleteId" @update:open="(open: boolean) => { if (!open) confirmDeleteId = null }">
      <DialogContent class="max-w-sm">
        <DialogHeader>
          <DialogTitle>Delete Draft</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete this local playbook draft? This cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter class="flex-row gap-3 sm:flex-row">
          <Button variant="outline" class="flex-1" @click="confirmDeleteId = null">Cancel</Button>
          <Button variant="destructive" class="flex-1" :disabled="!!deletingLocalId" @click="confirmDeleteLocalPlaybook">
            {{ deletingLocalId ? "Deleting..." : "Delete" }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Suggest broker dialog -->
    <Dialog :open="showSuggestDialog" @update:open="(open: boolean) => { if (!open) closeSuggestDialog() }">
      <DialogContent class="max-w-md">
        <DialogHeader>
          <DialogTitle>Suggest a Broker</DialogTitle>
          <DialogDescription>
            Know a data broker that's not in our registry? Let us know and we'll add it.
          </DialogDescription>
        </DialogHeader>

        <template v-if="suggestSuccess">
          <p class="py-4 text-center text-sm text-green-600 font-medium">
            Thanks! Your suggestion has been submitted.
          </p>
          <DialogFooter>
            <Button class="w-full" @click="closeSuggestDialog">Done</Button>
          </DialogFooter>
        </template>

        <template v-else>
          <div class="space-y-3 py-2">
            <div>
              <label class="mb-1 block text-sm font-medium">Broker name</label>
              <Input v-model="suggestName" placeholder="e.g. Spokeo" />
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium">Website URL</label>
              <Input v-model="suggestUrl" type="url" placeholder="https://..." />
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium">Notes <span class="font-normal text-muted-foreground">(optional)</span></label>
              <textarea
                v-model="suggestNotes"
                rows="2"
                class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                placeholder="Opt-out page URL, any details that might help..."
              />
            </div>
            <p v-if="suggestError" class="text-sm text-destructive">{{ suggestError }}</p>
          </div>
          <DialogFooter class="flex-row gap-3 sm:flex-row">
            <Button variant="outline" class="flex-1" @click="closeSuggestDialog">Cancel</Button>
            <Button
              class="flex-1"
              :disabled="suggestSubmitting || !suggestName.trim() || !suggestUrl.trim()"
              @click="handleSuggestBroker"
            >
              {{ suggestSubmitting ? "Submitting..." : "Submit" }}
            </Button>
          </DialogFooter>
        </template>
      </DialogContent>
    </Dialog>
  </div>
</template>
