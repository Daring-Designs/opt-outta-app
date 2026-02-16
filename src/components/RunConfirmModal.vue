<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { usePlaybooksStore } from "../stores/playbooks";
import type { Broker, PlaybookSummary, LocalPlaybook } from "../types";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ThumbsUp, ThumbsDown, CheckCircle, XCircle, FileText, Globe } from "lucide-vue-next";

const props = defineProps<{
  brokers: Broker[];
}>();

const emit = defineEmits<{
  (e: "confirm", playbookSelections: Record<string, string>): void;
  (e: "cancel"): void;
}>();

const playbooksStore = usePlaybooksStore();

// broker_id -> list of available community playbooks
const playbooksByBroker = ref<Map<string, PlaybookSummary[]>>(new Map());
// broker_id -> selected value: "best" | specific playbook id | "local:{id}"
const selections = ref<Record<string, string>>({});
const loading = ref(true);

onMounted(async () => {
  // Load local playbooks
  await playbooksStore.loadLocalPlaybooks();

  // Fetch community playbooks for all selected brokers in parallel
  const results = await Promise.allSettled(
    props.brokers.map(async (b) => {
      const list = await invoke<PlaybookSummary[]>("fetch_playbooks", {
        brokerId: b.id,
      });
      return { brokerId: b.id, playbooks: list };
    })
  );

  for (const result of results) {
    if (result.status === "fulfilled" && result.value.playbooks.length > 0) {
      playbooksByBroker.value.set(
        result.value.brokerId,
        result.value.playbooks
      );
    }
  }

  // Initialize selections: auto-select best community playbook, or first local, or leave empty
  for (const b of props.brokers) {
    if (playbooksByBroker.value.has(b.id)) {
      selections.value[b.id] = "best";
    } else {
      const locals = playbooksStore.getLocalPlaybooksForBroker(b.id);
      if (locals.length > 0) {
        selections.value[b.id] = `local:${locals[0].id}`;
      } else {
        selections.value[b.id] = "";
      }
    }
  }

  loading.value = false;
});

function getLocalPlaybooks(brokerId: string): LocalPlaybook[] {
  return playbooksStore.getLocalPlaybooksForBroker(brokerId);
}

function hasAnyPlaybook(brokerId: string): boolean {
  return (
    playbooksByBroker.value.has(brokerId) ||
    getLocalPlaybooks(brokerId).length > 0
  );
}

const brokersWithPlaybooks = computed(() =>
  props.brokers.filter((b) => hasAnyPlaybook(b.id))
);

const brokersWithoutPlaybooks = computed(() =>
  props.brokers.filter((b) => !hasAnyPlaybook(b.id))
);

const allHavePlaybooks = computed(() =>
  props.brokers.every((b) => selections.value[b.id] && selections.value[b.id] !== "")
);

function handleConfirm() {
  const playbookSelections: Record<string, string> = {};
  for (const [brokerId, selection] of Object.entries(selections.value)) {
    if (selection) {
      playbookSelections[brokerId] = selection;
    }
  }
  emit("confirm", playbookSelections);
}

function successRate(pb: PlaybookSummary): number | null {
  const total = pb.success_count + pb.failure_count;
  return total > 0 ? Math.round((pb.success_count / total) * 100) : null;
}

function isSelected(brokerId: string, value: string): boolean {
  return selections.value[brokerId] === value;
}

function select(brokerId: string, value: string) {
  selections.value[brokerId] = value;
}
</script>

<template>
  <Dialog :open="true" @update:open="(open: boolean) => { if (!open) $emit('cancel') }">
    <DialogContent class="max-w-xl max-h-[85vh] flex flex-col">
      <!-- Header -->
      <DialogHeader>
        <DialogTitle>Start Opt-Out Run</DialogTitle>
        <DialogDescription>
          {{ brokers.length }} broker{{ brokers.length > 1 ? "s" : "" }} selected
        </DialogDescription>
      </DialogHeader>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto -mx-1 px-1">
        <div v-if="loading" class="py-8 text-center text-sm text-muted-foreground">
          Checking for playbooks...
        </div>

        <template v-else>
          <div v-for="broker in brokersWithPlaybooks" :key="broker.id" class="mb-4">
            <div class="mb-2 flex items-center justify-between">
              <span class="text-sm font-medium">{{ broker.name }}</span>
              <span class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">{{ broker.difficulty }}</span>
            </div>

            <div class="space-y-1.5">
              <!-- Community: "Best rated" option -->
              <button
                v-if="playbooksByBroker.has(broker.id)"
                class="flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors"
                :class="isSelected(broker.id, 'best')
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:bg-muted/50'"
                @click="select(broker.id, 'best')"
              >
                <div class="flex h-4 w-4 flex-shrink-0 items-center justify-center rounded-full border-2"
                  :class="isSelected(broker.id, 'best') ? 'border-primary bg-primary' : 'border-muted-foreground/30'"
                >
                  <div v-if="isSelected(broker.id, 'best')" class="h-1.5 w-1.5 rounded-full bg-primary-foreground" />
                </div>
                <Globe class="h-4 w-4 flex-shrink-0 text-muted-foreground" />
                <div class="min-w-0 flex-1">
                  <p class="text-sm font-medium">Best rated community playbook</p>
                  <p class="text-xs text-muted-foreground">Auto-selects the highest scored playbook</p>
                </div>
              </button>

              <!-- Community playbook rows -->
              <button
                v-for="pb in playbooksByBroker.get(broker.id)"
                :key="pb.id"
                class="flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors"
                :class="isSelected(broker.id, pb.id)
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:bg-muted/50'"
                @click="select(broker.id, pb.id)"
              >
                <div class="flex h-4 w-4 flex-shrink-0 items-center justify-center rounded-full border-2"
                  :class="isSelected(broker.id, pb.id) ? 'border-primary bg-primary' : 'border-muted-foreground/30'"
                >
                  <div v-if="isSelected(broker.id, pb.id)" class="h-1.5 w-1.5 rounded-full bg-primary-foreground" />
                </div>
                <Globe class="h-4 w-4 flex-shrink-0 text-blue-500" />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <p class="text-sm font-medium">v{{ pb.version }}</p>
                    <span class="text-xs text-muted-foreground">{{ pb.steps_count }} steps</span>
                  </div>
                  <div class="mt-0.5 flex items-center gap-3 text-xs text-muted-foreground">
                    <span class="flex items-center gap-1">
                      <ThumbsUp class="h-3 w-3" />{{ pb.upvotes }}
                    </span>
                    <span class="flex items-center gap-1">
                      <ThumbsDown class="h-3 w-3" />{{ pb.downvotes }}
                    </span>
                    <span v-if="successRate(pb) !== null" class="flex items-center gap-1">
                      <CheckCircle class="h-3 w-3 text-green-500" />{{ successRate(pb) }}%
                    </span>
                    <span v-if="pb.failure_count > 0" class="flex items-center gap-1">
                      <XCircle class="h-3 w-3 text-red-500" />{{ pb.failure_count }}
                    </span>
                  </div>
                  <p v-if="pb.notes" class="mt-0.5 truncate text-xs text-muted-foreground">{{ pb.notes }}</p>
                </div>
              </button>

              <!-- Local playbook rows -->
              <button
                v-for="lp in getLocalPlaybooks(broker.id)"
                :key="'local:' + lp.id"
                class="flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors"
                :class="isSelected(broker.id, 'local:' + lp.id)
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:bg-muted/50'"
                @click="select(broker.id, 'local:' + lp.id)"
              >
                <div class="flex h-4 w-4 flex-shrink-0 items-center justify-center rounded-full border-2"
                  :class="isSelected(broker.id, 'local:' + lp.id) ? 'border-primary bg-primary' : 'border-muted-foreground/30'"
                >
                  <div v-if="isSelected(broker.id, 'local:' + lp.id)" class="h-1.5 w-1.5 rounded-full bg-primary-foreground" />
                </div>
                <FileText class="h-4 w-4 flex-shrink-0 text-orange-500" />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <p class="text-sm font-medium">{{ lp.title || "Untitled" }}</p>
                    <span class="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">LOCAL</span>
                  </div>
                  <p class="mt-0.5 text-xs text-muted-foreground">{{ lp.steps.length }} steps</p>
                </div>
              </button>
            </div>
          </div>

          <!-- Brokers without playbooks -->
          <div v-if="brokersWithoutPlaybooks.length > 0">
            <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-amber-600">
              No Playbook Available
            </h3>
            <p class="mb-2 text-xs text-muted-foreground">
              These brokers will be skipped. Record a playbook first from the Brokers page.
            </p>
            <div class="space-y-1">
              <div
                v-for="broker in brokersWithoutPlaybooks"
                :key="broker.id"
                class="flex items-center justify-between rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 dark:border-amber-800 dark:bg-amber-900/20"
              >
                <span class="text-sm">{{ broker.name }}</span>
                <span class="text-xs text-amber-600">no playbook</span>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- Footer -->
      <DialogFooter class="flex-row items-center justify-between sm:justify-between">
        <Button variant="outline" @click="$emit('cancel')">
          Cancel
        </Button>
        <div class="flex items-center gap-3">
          <span
            v-if="!loading && !allHavePlaybooks"
            class="text-xs text-amber-600"
          >
            {{ brokersWithoutPlaybooks.length }} broker{{ brokersWithoutPlaybooks.length > 1 ? "s" : "" }} will be skipped
          </span>
          <Button
            :disabled="loading || brokersWithPlaybooks.length === 0"
            @click="handleConfirm"
          >
            Start Run
          </Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
