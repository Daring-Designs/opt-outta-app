<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { useProfileStore } from "../stores/profile";
import { useBrokersStore } from "../stores/brokers";
import { useHistoryStore } from "../stores/history";
import { useOptOutStore } from "../stores/optout";
import OptOutRunner from "../components/OptOutRunner.vue";
import UserActionModal from "../components/UserActionModal.vue";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

const router = useRouter();
const profileStore = useProfileStore();
const brokersStore = useBrokersStore();
const historyStore = useHistoryStore();
const optOutStore = useOptOutStore();

onMounted(async () => {
  if (!profileStore.loaded) await profileStore.loadProfile();
  if (brokersStore.brokers.length === 0) await brokersStore.loadBrokers();
  await historyStore.loadHistory();
  await optOutStore.setupListeners();
});
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

    <!-- Quick Actions -->
    <h2 class="mb-3 text-lg font-semibold">Quick Actions</h2>
    <div class="flex flex-wrap gap-3">
      <Button variant="outline" @click="router.push('/profile')">
        Set Up Profile
      </Button>
      <Button variant="outline" @click="router.push('/brokers')">
        Browse Brokers
      </Button>
    </div>

    <!-- Modals -->
    <UserActionModal />
    <OptOutRunner />
  </div>
</template>
