<script setup lang="ts">
import { ref, computed, watch, nextTick } from "vue";
import { usePlaybooksStore } from "../stores/playbooks";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { ChevronUp, ChevronDown, X, Check } from "lucide-vue-next";

const store = usePlaybooksStore();
const playbookTitle = ref("");
const notes = ref("");
const submitting = ref(false);
const submitError = ref<string | null>(null);
const submitSuccess = ref<string | null>(null);
const saving = ref(false);
const saveSuccess = ref(false);
const stepListEl = ref<HTMLElement | null>(null);

const isActive = computed(
  () => store.isRecording || store.isReviewing || store.recordingStatus === "submitting"
);

const profileKeys = [
  { value: "firstName", label: "First Name" },
  { value: "lastName", label: "Last Name" },
  { value: "email", label: "Email" },
  { value: "phone", label: "Phone" },
  { value: "address", label: "Address" },
  { value: "city", label: "City" },
  { value: "state", label: "State" },
  { value: "zip", label: "ZIP" },
  { value: "dob", label: "Date of Birth" },
  { value: "fullName", label: "Full Name" },
];

function actionBadgeClass(action: string): string {
  switch (action) {
    case "fill":
      return "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400";
    case "click":
      return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400";
    case "navigate":
      return "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400";
    case "captcha":
      return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400";
    case "user_prompt":
      return "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400";
    case "select":
      return "bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-400";
    case "check":
      return "bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-400";
    case "wait":
    case "wait_for":
      return "bg-muted text-muted-foreground";
    default:
      return "bg-muted text-muted-foreground";
  }
}

function actionDisplayName(action: string): string {
  if (action === "user_prompt") return "manual prompt";
  return action;
}

function moveStep(index: number, direction: -1 | 1) {
  const target = index + direction;
  if (target < 0 || target >= store.editableSteps.length) return;
  store.reorderStep(index, target);
}

// Auto-scroll to bottom when new steps arrive during recording
watch(
  () => store.editableSteps.length,
  () => {
    if (store.isRecording && stepListEl.value) {
      nextTick(() => {
        stepListEl.value!.scrollTop = stepListEl.value!.scrollHeight;
      });
    }
  }
);

async function handleSubmit() {
  submitting.value = true;
  submitError.value = null;
  try {
    const message = await store.submitPlaybook(playbookTitle.value || null, notes.value || null);
    submitSuccess.value = message;
  } catch (e) {
    submitError.value = String(e);
  } finally {
    submitting.value = false;
  }
}

async function handleSaveLocally() {
  saving.value = true;
  submitError.value = null;
  try {
    if (store.editingLocalId) {
      await store.updateLocalPlaybook(
        store.editingLocalId,
        playbookTitle.value || null,
        notes.value || null
      );
    } else {
      await store.saveLocally(playbookTitle.value || null, notes.value || null);
    }
    saveSuccess.value = true;
    setTimeout(() => {
      saveSuccess.value = false;
    }, 2000);
    playbookTitle.value = "";
    notes.value = "";
  } catch (e) {
    submitError.value = String(e);
  } finally {
    saving.value = false;
  }
}

function handleDiscard() {
  playbookTitle.value = "";
  notes.value = "";
  submitError.value = null;
  store.resetRecording();
}

function handleSuccessDone() {
  submitSuccess.value = null;
  playbookTitle.value = "";
  notes.value = "";
}
</script>

<template>
  <!-- Success modal -->
  <Dialog :open="!!submitSuccess" @update:open="(open: boolean) => { if (!open) handleSuccessDone() }">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <div class="flex items-center gap-3">
          <div class="flex h-10 w-10 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30">
            <Check class="h-5 w-5 text-green-600" />
          </div>
          <DialogTitle>Playbook Submitted</DialogTitle>
        </div>
      </DialogHeader>
      <DialogDescription>{{ submitSuccess }}</DialogDescription>
      <DialogFooter>
        <Button class="w-full" @click="handleSuccessDone">
          Done
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>

  <!-- Full-screen recording/review overlay -->
  <div
    v-if="isActive && !submitSuccess"
    class="fixed inset-0 z-50 flex flex-col bg-background"
  >
    <!-- Header -->
    <div class="border-b border-border px-6 py-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div
            v-if="store.isRecording"
            class="h-3 w-3 animate-pulse rounded-full bg-red-500"
          />
          <h2 class="text-lg font-semibold">
            <template v-if="store.isRecording">
              Recording {{ store.recordingBrokerName }}
            </template>
            <template v-else>
              Review Recorded Steps
            </template>
          </h2>
          <span class="rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground">
            {{ store.editableSteps.length }} step{{ store.editableSteps.length !== 1 ? "s" : "" }}
          </span>
        </div>
        <span v-if="store.isReviewing" class="text-sm text-muted-foreground">
          {{ store.recordingBrokerName }}
        </span>
      </div>
      <Input
        v-if="store.isReviewing"
        v-model="playbookTitle"
        type="text"
        class="mt-3"
        placeholder="Playbook title (e.g., 'Standard opt-out via web form')"
      />
    </div>

    <!-- Scrollable step list -->
    <div ref="stepListEl" class="flex-1 overflow-y-auto px-6 py-4">
      <div
        v-for="(step, i) in store.editableSteps"
        :key="i"
        class="mb-3 rounded-lg border border-border p-3"
      >
        <div class="mb-2 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="font-mono text-xs text-muted-foreground">{{ step.position }}</span>
            <span
              class="rounded px-2 py-0.5 text-xs font-medium"
              :class="actionBadgeClass(step.action)"
            >
              {{ actionDisplayName(step.action) }}
            </span>
            <span
              v-if="step.selector"
              class="max-w-[200px] truncate font-mono text-xs text-muted-foreground"
            >
              {{ step.selector }}
            </span>
          </div>
          <div class="flex items-center gap-1">
            <button
              class="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
              :disabled="i === 0"
              @click="moveStep(i, -1)"
            >
              <ChevronUp class="h-4 w-4" />
            </button>
            <button
              class="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
              :disabled="i === store.editableSteps.length - 1"
              @click="moveStep(i, 1)"
            >
              <ChevronDown class="h-4 w-4" />
            </button>
            <button
              class="rounded p-1 text-destructive/70 hover:bg-destructive/10 hover:text-destructive"
              @click="store.deleteStep(i)"
            >
              <X class="h-4 w-4" />
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <input
            :value="step.description"
            type="text"
            class="w-full rounded-md border border-input bg-background px-2 py-1 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            :placeholder="step.action === 'user_prompt' ? 'Step title (e.g., Find your profile)' : 'Step description'"
            @input="store.updateStep(i, { description: ($event.target as HTMLInputElement).value })"
          />

          <div v-if="step.action === 'user_prompt'">
            <p class="mb-1 text-xs text-muted-foreground">During playback, the user will be prompted to complete this step manually.</p>
            <textarea
              :value="step.value"
              class="w-full rounded-md border border-input bg-background px-2 py-1 text-sm ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
              rows="2"
              placeholder="Instruction for the user (e.g., 'Search for your name and find your profile URL')"
              @input="store.updateStep(i, { value: ($event.target as HTMLTextAreaElement).value })"
            />
          </div>

          <div v-if="step.action === 'fill' || step.action === 'select'" class="space-y-2">
            <div class="flex items-center gap-2">
              <label class="text-xs text-muted-foreground">Value source:</label>
              <select
                :value="step.profile_key ?? '__manual__'"
                class="rounded-md border border-input bg-background px-2 py-1 text-xs ring-offset-background focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
                @change="(() => {
                  const v = ($event.target as HTMLSelectElement).value;
                  if (v === '__manual__') {
                    store.updateStep(i, { profile_key: null });
                  } else {
                    store.updateStep(i, { profile_key: v });
                  }
                })()"
              >
                <option value="__manual__">Manual</option>
                <option
                  v-for="pk in profileKeys"
                  :key="pk.value"
                  :value="pk.value"
                >
                  {{ pk.label }}
                </option>
              </select>
            </div>
            <p v-if="!step.profile_key" class="text-xs text-muted-foreground">
              During playback, the field will be highlighted and you'll be prompted to fill it out in the browser.
            </p>
          </div>

          <label class="flex items-center gap-2">
            <input
              type="checkbox"
              :checked="step.optional"
              class="rounded border-input text-primary"
              @change="store.updateStep(i, { optional: ($event.target as HTMLInputElement).checked })"
            />
            <span class="text-xs text-muted-foreground">Optional (skip if element not found)</span>
          </label>
        </div>
      </div>

      <!-- Add User Prompt button -->
      <button
        class="mt-2 w-full rounded-lg border border-dashed border-purple-300 px-4 py-2 text-sm font-medium text-purple-600 hover:border-purple-400 hover:bg-purple-50 dark:border-purple-700 dark:text-purple-400 dark:hover:border-purple-600 dark:hover:bg-purple-900/20"
        @click="store.addUserPromptStep()"
      >
        + Add Manual Prompt
      </button>

      <!-- Empty states -->
      <p
        v-if="store.editableSteps.length === 0 && store.isRecording"
        class="py-8 text-center text-sm text-muted-foreground"
      >
        Waiting for actions...
      </p>
      <p
        v-if="store.editableSteps.length === 0 && store.isReviewing"
        class="py-8 text-center text-sm text-muted-foreground"
      >
        No steps recorded. Try recording again.
      </p>
    </div>

    <!-- Bottom bar -->
    <div class="border-t border-border px-6 py-4">
      <!-- Recording controls -->
      <div v-if="store.isRecording" class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <div class="h-2 w-2 animate-pulse rounded-full bg-red-500" />
          <span class="text-sm text-red-600">Recording in progress</span>
        </div>
        <div class="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            class="border-yellow-400 bg-yellow-50 text-yellow-800 hover:bg-yellow-100 dark:border-yellow-700 dark:bg-yellow-900/20 dark:text-yellow-400 dark:hover:bg-yellow-900/40"
            @click="store.markCaptcha()"
          >
            Mark CAPTCHA
          </Button>
          <Button
            variant="outline"
            size="sm"
            class="border-purple-400 bg-purple-50 text-purple-800 hover:bg-purple-100 dark:border-purple-700 dark:bg-purple-900/20 dark:text-purple-400 dark:hover:bg-purple-900/40"
            @click="store.markUserPrompt()"
          >
            Mark Manual Step
          </Button>
          <Button
            variant="destructive"
            @click="store.stopRecording()"
          >
            Stop Recording
          </Button>
        </div>
      </div>

      <!-- Review controls -->
      <div v-else>
        <textarea
          v-model="notes"
          class="mb-3 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
          rows="2"
          placeholder="Notes about this playbook (e.g., 'Works as of Feb 2026, has one CAPTCHA')"
        />

        <p v-if="submitError" class="mb-3 text-sm text-destructive">{{ submitError }}</p>

        <div class="flex items-center justify-between">
          <Button variant="outline" @click="handleDiscard">
            Discard
          </Button>
          <div class="flex items-center gap-2">
            <span
              v-if="saveSuccess"
              class="text-sm font-medium text-green-600"
            >
              Saved!
            </span>
            <Button
              variant="outline"
              class="border-green-300 text-green-700 hover:bg-green-50 dark:border-green-700 dark:text-green-400 dark:hover:bg-green-900/20"
              :disabled="saving || store.editableSteps.length === 0"
              @click="handleSaveLocally"
            >
              {{ saving ? "Saving..." : store.editingLocalId ? "Update Draft" : "Save Locally" }}
            </Button>
            <Button
              :disabled="submitting || store.editableSteps.length === 0"
              @click="handleSubmit"
            >
              {{ submitting ? "Submitting..." : "Submit to Community" }}
            </Button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
