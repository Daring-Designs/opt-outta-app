<script setup lang="ts">
import { ref } from "vue";
import { useProfileStore } from "../stores/profile";
import { useThemeStore, type ThemeMode } from "../stores/theme";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Sun, Moon, Monitor } from "lucide-vue-next";

const profileStore = useProfileStore();
const themeStore = useThemeStore();

const confirmDelete = ref(false);

const themeOptions: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
];

async function deleteAllData() {
  await profileStore.deleteProfile();
  confirmDelete.value = false;
}
</script>

<template>
  <div class="mx-auto max-w-2xl">
    <h1 class="mb-6 text-2xl font-bold">Settings</h1>

    <!-- Appearance -->
    <Card class="mb-6">
      <CardHeader>
        <CardTitle class="text-base">Appearance</CardTitle>
        <CardDescription>Choose how the app looks.</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="inline-flex rounded-lg border border-border p-1">
          <button
            v-for="opt in themeOptions"
            :key="opt.value"
            class="inline-flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium transition-colors"
            :class="
              themeStore.mode === opt.value
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:text-foreground'
            "
            @click="themeStore.setMode(opt.value)"
          >
            <component :is="opt.icon" class="h-4 w-4" />
            {{ opt.label }}
          </button>
        </div>
      </CardContent>
    </Card>

    <!-- Danger Zone -->
    <Card class="border-destructive/50">
      <CardHeader>
        <CardTitle class="text-base text-destructive">Danger Zone</CardTitle>
        <CardDescription>
          Permanently delete all local data including your profile and playbooks.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div v-if="!confirmDelete">
          <Button variant="outline" class="border-destructive/50 text-destructive hover:bg-destructive/10" @click="confirmDelete = true">
            Delete All Data
          </Button>
        </div>
        <div v-else class="flex items-center gap-3">
          <Button variant="destructive" @click="deleteAllData">
            Yes, Delete Everything
          </Button>
          <Button variant="outline" @click="confirmDelete = false">
            Cancel
          </Button>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
