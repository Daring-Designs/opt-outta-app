<script setup lang="ts">
import { ref } from "vue";
import { useOptOutStore } from "../stores/optout";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Check, X, ClipboardCopy } from "lucide-vue-next";

const store = useOptOutStore();
const copied = ref(false);

async function handleCopyReport() {
  const ok = await store.copyReport();
  if (ok) {
    copied.value = true;
    setTimeout(() => (copied.value = false), 2000);
  }
}
</script>

<template>
  <!-- Active run: thin bottom bar -->
  <div
    v-if="store.isActive"
    class="fixed inset-x-0 bottom-0 z-40 border-t border-border bg-card p-4 shadow-lg"
  >
    <div class="mx-auto max-w-4xl">
      <div class="mb-3 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div
            v-if="store.status === 'running'"
            class="h-3 w-3 animate-pulse rounded-full bg-primary"
          />
          <div
            v-else-if="store.status === 'waiting_for_user'"
            class="h-3 w-3 animate-pulse rounded-full bg-yellow-500"
          />
          <span class="text-sm font-medium">
            {{ store.currentBrokerName || "Starting..." }}
          </span>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-xs text-muted-foreground">
            {{ store.brokersCompleted }}/{{ store.brokersTotal }} brokers
          </span>
          <Button
            variant="outline"
            size="sm"
            class="border-destructive/50 text-destructive hover:bg-destructive/10"
            @click="store.cancelRun()"
          >
            Cancel
          </Button>
        </div>
      </div>
      <div class="mb-2 h-2 overflow-hidden rounded-full bg-muted">
        <div
          class="h-full rounded-full bg-primary transition-all duration-500"
          :class="{ 'bg-yellow-500': store.status === 'waiting_for_user' }"
          :style="{ width: store.progressPercent + '%' }"
        />
      </div>
      <p class="text-xs text-muted-foreground">{{ store.currentStep }}</p>
      <p v-if="store.error" class="mt-1 text-xs text-destructive">{{ store.error }}</p>
    </div>
  </div>

  <!-- Completed/failed: Dialog results panel -->
  <Dialog
    :open="store.status === 'completed' || store.status === 'failed'"
    @update:open="(open: boolean) => { if (!open) store.reset() }"
  >
    <DialogContent class="max-w-lg">
      <DialogHeader>
        <div class="flex items-center gap-3">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-full"
            :class="store.status === 'completed' ? 'bg-green-100 dark:bg-green-900/30' : 'bg-red-100 dark:bg-red-900/30'"
          >
            <Check v-if="store.status === 'completed'" class="h-5 w-5 text-green-600" />
            <X v-else class="h-5 w-5 text-red-600" />
          </div>
          <div>
            <DialogTitle>
              {{ store.status === "completed" ? "Run Complete" : "Run Failed" }}
            </DialogTitle>
            <DialogDescription>
              {{ store.lastResult?.succeeded ?? 0 }} of {{ store.lastResult?.total ?? 0 }} brokers succeeded
            </DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <!-- Broker results list -->
      <div class="max-h-[50vh] overflow-y-auto">
        <div
          v-for="outcome in store.brokerOutcomes"
          :key="outcome.brokerId"
          class="flex items-start gap-3 border-b border-border py-3 last:border-0"
        >
          <div class="mt-0.5 flex-shrink-0">
            <Check v-if="outcome.succeeded" class="h-5 w-5 text-green-500" />
            <X v-else class="h-5 w-5 text-red-400" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium">{{ outcome.brokerName }}</p>
            <p v-if="outcome.error" class="mt-0.5 truncate text-xs text-destructive">
              {{ outcome.error }}
            </p>
            <p v-else-if="!outcome.succeeded" class="mt-0.5 text-xs text-muted-foreground">
              {{ outcome.lastStep }}
            </p>
          </div>
        </div>

        <p
          v-if="store.brokerOutcomes.length === 0"
          class="py-4 text-center text-sm text-muted-foreground"
        >
          No broker results recorded.
        </p>
      </div>

      <!-- Footer actions -->
      <DialogFooter class="flex-row justify-between sm:justify-between">
        <Button variant="outline" @click="handleCopyReport">
          <ClipboardCopy class="h-4 w-4" />
          {{ copied ? "Copied!" : "Copy Report" }}
        </Button>
        <Button @click="store.reset()">
          Done
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
