<script setup lang="ts">
import { useOptOutStore } from "../stores/optout";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Info, AlertTriangle, XCircle } from "lucide-vue-next";

const store = useOptOutStore();
</script>

<template>
  <Dialog
    :open="!!store.actionRequired"
    @update:open="(open: boolean) => { if (!open) store.cancelRun() }"
  >
    <DialogContent class="max-w-md">
      <DialogHeader class="items-center text-center sm:items-center sm:text-center">
        <!-- Step Failed icon -->
        <div
          v-if="store.actionRequired?.type === 'step_failed'"
          class="flex h-12 w-12 items-center justify-center rounded-full bg-red-100 dark:bg-red-900/30"
        >
          <XCircle class="h-6 w-6 text-red-600" />
        </div>
        <!-- User Prompt icon -->
        <div
          v-else-if="store.actionRequired?.type === 'user_prompt'"
          class="flex h-12 w-12 items-center justify-center rounded-full bg-blue-100 dark:bg-blue-900/30"
        >
          <Info class="h-6 w-6 text-blue-600" />
        </div>
        <!-- Default (captcha etc.) icon -->
        <div v-else class="flex h-12 w-12 items-center justify-center rounded-full bg-yellow-100 dark:bg-yellow-900/30">
          <AlertTriangle class="h-6 w-6 text-yellow-600" />
        </div>

        <DialogTitle>
          {{ store.actionRequired?.type === 'step_failed'
            ? 'Step Failed'
            : store.actionRequired?.type === 'user_prompt'
              ? 'Manual Step Required'
              : 'Action Required' }}
        </DialogTitle>
        <DialogDescription>
          {{ store.actionRequired?.message }}
        </DialogDescription>
      </DialogHeader>

      <!-- Step failure detail box -->
      <div
        v-if="store.actionRequired?.type === 'step_failed'"
        class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 dark:border-red-800 dark:bg-red-900/20 dark:text-red-300"
      >
        <span class="font-medium">Step {{ store.actionRequired.step_position }}:</span>
        {{ store.actionRequired.step_description }}
      </div>

      <!-- Detailed instructions for user prompts -->
      <div
        v-else-if="store.actionRequired?.description"
        class="rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-800 dark:border-blue-800 dark:bg-blue-900/20 dark:text-blue-300"
      >
        {{ store.actionRequired.description }}
      </div>

      <!-- Step failed: three-button footer -->
      <DialogFooter v-if="store.actionRequired?.type === 'step_failed'" class="flex-row gap-3 sm:flex-row">
        <Button variant="outline" class="flex-1" @click="store.abortBroker()">
          Abort
        </Button>
        <Button variant="outline" class="flex-1" @click="store.skipFailedStep()">
          Skip
        </Button>
        <Button class="flex-1" @click="store.retryFailedStep()">
          Retry
        </Button>
      </DialogFooter>

      <!-- Normal user actions: two-button footer -->
      <DialogFooter v-else class="flex-row gap-3 sm:flex-row">
        <Button variant="outline" class="flex-1" @click="store.cancelRun()">
          Cancel Run
        </Button>
        <Button class="flex-1" @click="store.continueAfterUserAction()">
          {{ store.actionRequired?.type === 'user_prompt' ? "I've completed this step" : 'Done' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
