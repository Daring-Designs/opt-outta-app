<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { usePlaybooksStore } from "../stores/playbooks";
import type { Broker, PlaybookSummary, LocalPlaybook } from "../types";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

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

function selectAllBest() {
  for (const b of brokersWithPlaybooks.value) {
    if (playbooksByBroker.value.has(b.id)) {
      selections.value[b.id] = "best";
    }
  }
}

function handleConfirm() {
  // Build the playbook selections map
  const playbookSelections: Record<string, string> = {};
  for (const [brokerId, selection] of Object.entries(selections.value)) {
    if (selection) {
      playbookSelections[brokerId] = selection;
    }
  }
  emit("confirm", playbookSelections);
}

function formatScore(pb: PlaybookSummary): string {
  const net = pb.upvotes - pb.downvotes;
  const rate =
    pb.success_count + pb.failure_count > 0
      ? Math.round(
          (pb.success_count / (pb.success_count + pb.failure_count)) * 100
        )
      : null;
  let s = `${net >= 0 ? "+" : ""}${net} votes`;
  if (rate !== null) s += ` \u00b7 ${rate}% success`;
  return s;
}
</script>

<template>
  <Dialog :open="true" @update:open="(open: boolean) => { if (!open) $emit('cancel') }">
    <DialogContent class="max-w-xl max-h-[85vh] flex flex-col">
      <!-- Header -->
      <DialogHeader>
        <div class="flex items-center justify-between">
          <div>
            <DialogTitle>Start Opt-Out Run</DialogTitle>
            <DialogDescription>
              {{ brokers.length }} broker{{ brokers.length > 1 ? "s" : "" }} selected
            </DialogDescription>
          </div>
          <div v-if="!loading && brokersWithPlaybooks.length > 1">
            <Button variant="outline" size="sm" @click="selectAllBest">
              Use All Best
            </Button>
          </div>
        </div>
      </DialogHeader>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto">
        <div v-if="loading" class="py-8 text-center text-sm text-muted-foreground">
          Checking for playbooks...
        </div>

        <template v-else>
          <!-- Brokers with playbooks -->
          <div v-if="brokersWithPlaybooks.length > 0" class="mb-4">
            <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Playbooks Available
            </h3>
            <div class="space-y-2">
              <div
                v-for="broker in brokersWithPlaybooks"
                :key="broker.id"
                class="rounded-lg border border-border p-3"
              >
                <div class="mb-2 flex items-center justify-between">
                  <span class="text-sm font-medium">{{
                    broker.name
                  }}</span>
                  <span class="text-xs text-muted-foreground">{{ broker.difficulty }}</span>
                </div>
                <select
                  :value="selections[broker.id]"
                  class="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                  @change="
                    selections[broker.id] = (
                      $event.target as HTMLSelectElement
                    ).value
                  "
                >
                  <!-- Community playbooks -->
                  <option
                    v-if="playbooksByBroker.has(broker.id)"
                    value="best"
                  >
                    Best rated community playbook
                  </option>
                  <option
                    v-for="pb in playbooksByBroker.get(broker.id)"
                    :key="pb.id"
                    :value="pb.id"
                  >
                    v{{ pb.version }} — {{ formatScore(pb) }}
                    {{ pb.notes ? ` — ${pb.notes.slice(0, 50)}` : "" }}
                  </option>
                  <!-- Local playbooks -->
                  <option
                    v-for="lp in getLocalPlaybooks(broker.id)"
                    :key="'local:' + lp.id"
                    :value="'local:' + lp.id"
                  >
                    Local: {{ lp.title || "Untitled" }} — {{ lp.steps.length }} steps
                  </option>
                </select>
              </div>
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
