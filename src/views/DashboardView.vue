<script setup lang="ts">
import { onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { useProfileStore } from "../stores/profile";
import { useBrokersStore } from "../stores/brokers";
import { useHistoryStore } from "../stores/history";
import { useOptOutStore } from "../stores/optout";
import { usePlaybooksStore } from "../stores/playbooks";
import OptOutRunner from "../components/OptOutRunner.vue";
import UserActionModal from "../components/UserActionModal.vue";
import { Card, CardContent } from "@/components/ui/card";
import { Check, Circle } from "lucide-vue-next";

const router = useRouter();
const profileStore = useProfileStore();
const brokersStore = useBrokersStore();
const historyStore = useHistoryStore();
const optOutStore = useOptOutStore();
const playbooksStore = usePlaybooksStore();

onMounted(async () => {
  if (!profileStore.loaded) await profileStore.loadProfile();
  if (brokersStore.brokers.length === 0) await brokersStore.loadBrokers();
  await historyStore.loadHistory();
  await playbooksStore.loadLocalPlaybooks();
  await playbooksStore.loadTrackedSubmissions();
  await optOutStore.setupListeners();
  // Check for status updates on pending submissions (fire and forget)
  playbooksStore.refreshSubmissionStatuses();
});

// Roadmap step completion logic
const hasProfile = computed(() => profileStore.completeness >= 80);
const hasRunOptOut = computed(() => historyStore.submissions.length > 0);
const hasConfirmed = computed(() => historyStore.confirmedCount > 0);
const hasCreatedPlaybook = computed(() => playbooksStore.localPlaybooks.length > 0);
const hasSubmittedPlaybook = computed(() => playbooksStore.hasSubmitted);
const hasApprovedPlaybook = computed(() => playbooksStore.hasApproved);

interface RoadmapStep {
  label: string;
  description: string;
  done: boolean;
  action: () => void;
}

const roadmapSteps = computed<RoadmapStep[]>(() => [
  {
    label: "Complete your profile",
    description: "Fill out your personal info so playbooks can auto-fill opt-out forms.",
    done: hasProfile.value,
    action: () => router.push("/profile"),
  },
  {
    label: "Run your first opt-out",
    description: "Select brokers and run a playbook to submit opt-out requests.",
    done: hasRunOptOut.value,
    action: () => router.push("/brokers"),
  },
  {
    label: "Confirm an opt-out",
    description: "Verify that a broker has removed your data after submitting.",
    done: hasConfirmed.value,
    action: () => router.push("/history"),
  },
  {
    label: "Create a playbook",
    description: "Record your own opt-out steps for a broker using the playbook recorder.",
    done: hasCreatedPlaybook.value,
    action: () => router.push("/brokers"),
  },
  {
    label: "Submit a playbook to the community",
    description: "Share your playbook so others can use it to opt out of the same broker.",
    done: hasSubmittedPlaybook.value,
    action: () => router.push("/brokers"),
  },
  {
    label: "Get a playbook approved",
    description: "Have a submitted playbook reviewed and approved for community use.",
    done: hasApprovedPlaybook.value,
    action: () => router.push("/brokers"),
  },
]);

const completedCount = computed(() => roadmapSteps.value.filter((s) => s.done).length);
const totalSteps = computed(() => roadmapSteps.value.length);
const roadmapProgress = computed(() => Math.round((completedCount.value / totalSteps.value) * 100));
</script>

<template>
  <div class="mx-auto max-w-4xl">
    <h1 class="mb-6 text-2xl font-bold">Dashboard</h1>

    <div class="mb-8 grid grid-cols-1 gap-4 sm:grid-cols-4">
      <!-- Profile Completeness -->
      <Card>
        <CardContent class="pt-5">
          <p class="mb-1 text-sm font-medium text-muted-foreground">Profile</p>
          <p class="text-3xl font-bold">{{ profileStore.completeness }}%</p>
          <div class="mt-3 h-2 overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary transition-all"
              :style="{ width: profileStore.completeness + '%' }"
            />
          </div>
        </CardContent>
      </Card>

      <!-- Submitted -->
      <Card>
        <CardContent class="pt-5">
          <p class="mb-1 text-sm font-medium text-muted-foreground">Pending</p>
          <p class="text-3xl font-bold text-yellow-600">{{ historyStore.pendingCount }}</p>
          <p class="mt-2 text-sm text-muted-foreground">awaiting confirmation</p>
        </CardContent>
      </Card>

      <!-- Confirmed -->
      <Card>
        <CardContent class="pt-5">
          <p class="mb-1 text-sm font-medium text-muted-foreground">Confirmed</p>
          <p class="text-3xl font-bold text-green-600">{{ historyStore.confirmedCount }}</p>
          <p class="mt-2 text-sm text-muted-foreground">opted out</p>
        </CardContent>
      </Card>

      <!-- Re-listing Alerts -->
      <Card>
        <CardContent class="pt-5">
          <p class="mb-1 text-sm font-medium text-muted-foreground">Alerts</p>
          <p class="text-3xl font-bold" :class="historyStore.relistCount > 0 ? 'text-red-600' : 'text-muted-foreground'">
            {{ historyStore.relistCount }}
          </p>
          <p class="mt-2 text-sm text-muted-foreground">re-listing checks due</p>
        </CardContent>
      </Card>
    </div>

    <!-- Roadmap -->
    <div class="mb-8">
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Getting Started</h2>
        <span class="text-sm text-muted-foreground">{{ completedCount }}/{{ totalSteps }} complete</span>
      </div>

      <div class="mb-4 h-2 overflow-hidden rounded-full bg-muted">
        <div
          class="h-full rounded-full bg-primary transition-all"
          :style="{ width: roadmapProgress + '%' }"
        />
      </div>

      <div class="space-y-1">
        <button
          v-for="(step, i) in roadmapSteps"
          :key="i"
          class="flex w-full items-start gap-3 rounded-lg px-3 py-3 text-left transition-colors hover:bg-muted/50"
          @click="step.action()"
        >
          <div class="mt-0.5 flex-shrink-0">
            <Check
              v-if="step.done"
              class="h-5 w-5 text-green-600"
            />
            <Circle
              v-else
              class="h-5 w-5 text-muted-foreground/40"
            />
          </div>
          <div>
            <p
              class="text-sm font-medium"
              :class="step.done ? 'text-muted-foreground line-through' : ''"
            >
              {{ step.label }}
            </p>
            <p class="text-xs text-muted-foreground">{{ step.description }}</p>
          </div>
        </button>
      </div>
    </div>

    <!-- Modals -->
    <UserActionModal />
    <OptOutRunner />
  </div>
</template>
