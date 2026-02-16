<script setup lang="ts">
import { ref, shallowRef, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useProfileStore } from "../stores/profile";
import { useThemeStore, type ThemeMode } from "../stores/theme";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Sun, Moon, Monitor, Download, RefreshCw, CheckCircle, AlertCircle, Loader2, FileText } from "lucide-vue-next";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import type { ChangelogEntry } from "../types";

const profileStore = useProfileStore();
const themeStore = useThemeStore();

const confirmDelete = ref(false);

// Changelog state
const changelogStatus = ref<"idle" | "loading" | "loaded" | "error">("idle");
const changelogEntries = ref<ChangelogEntry[]>([]);
const changelogError = ref("");

async function fetchChangelog() {
  changelogStatus.value = "loading";
  changelogError.value = "";
  try {
    changelogEntries.value = await invoke<ChangelogEntry[]>("fetch_changelog");
    changelogStatus.value = "loaded";
  } catch (e) {
    changelogStatus.value = "error";
    changelogError.value = e instanceof Error ? e.message : String(e);
  }
}

// Update state
const appVersion = ref("");
const updateStatus = ref<"idle" | "checking" | "available" | "downloading" | "installed" | "up-to-date" | "error">("idle");
const updateError = ref("");
const availableUpdate = shallowRef<Update | null>(null);
const downloadProgress = ref("");

onMounted(async () => {
  appVersion.value = await getVersion();
});

async function checkForUpdates() {
  updateStatus.value = "checking";
  updateError.value = "";
  availableUpdate.value = null;

  try {
    const update = await check();
    if (update) {
      availableUpdate.value = update;
      updateStatus.value = "available";
    } else {
      updateStatus.value = "up-to-date";
    }
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = e instanceof Error ? e.message : String(e);
  }
}

async function installUpdate() {
  if (!availableUpdate.value) return;

  updateStatus.value = "downloading";
  downloadProgress.value = "Downloading...";

  try {
    let downloaded = 0;
    let contentLength = 0;

    await availableUpdate.value.downloadAndInstall((event) => {
      if (event.event === "Started" && event.data.contentLength) {
        contentLength = event.data.contentLength;
      } else if (event.event === "Progress") {
        downloaded += event.data.chunkLength;
        if (contentLength > 0) {
          const pct = Math.round((downloaded / contentLength) * 100);
          downloadProgress.value = `Downloading... ${pct}%`;
        }
      } else if (event.event === "Finished") {
        downloadProgress.value = "Installing...";
      }
    });

    updateStatus.value = "installed";
  } catch (e) {
    updateStatus.value = "error";
    updateError.value = e instanceof Error ? e.message : String(e);
  }
}

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

    <!-- Updates -->
    <Card class="mb-6">
      <CardHeader>
        <CardTitle class="text-base">Updates</CardTitle>
        <CardDescription>
          Current version: <span class="font-mono">{{ appVersion || "..." }}</span>
        </CardDescription>
      </CardHeader>
      <CardContent>
        <!-- Idle -->
        <Button v-if="updateStatus === 'idle'" variant="outline" @click="checkForUpdates">
          <RefreshCw class="mr-2 h-4 w-4" />
          Check for Updates
        </Button>

        <!-- Checking -->
        <div v-else-if="updateStatus === 'checking'" class="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" />
          Checking for updates...
        </div>

        <!-- Update available -->
        <div v-else-if="updateStatus === 'available'" class="space-y-3">
          <p class="text-sm">
            Version <span class="font-mono font-medium">{{ availableUpdate?.version }}</span> is available.
          </p>
          <Button @click="installUpdate">
            <Download class="mr-2 h-4 w-4" />
            Download &amp; Install
          </Button>
        </div>

        <!-- Downloading / Installing -->
        <div v-else-if="updateStatus === 'downloading'" class="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" />
          {{ downloadProgress }}
        </div>

        <!-- Installed — restart required -->
        <div v-else-if="updateStatus === 'installed'" class="space-y-3">
          <div class="flex items-center gap-2 text-sm">
            <CheckCircle class="h-4 w-4 text-green-500" />
            <span>Update installed. Restart to apply.</span>
          </div>
          <Button @click="relaunch()">
            <RefreshCw class="mr-2 h-4 w-4" />
            Restart Now
          </Button>
        </div>

        <!-- Up to date -->
        <div v-else-if="updateStatus === 'up-to-date'" class="flex items-center gap-2 text-sm">
          <CheckCircle class="h-4 w-4 text-green-500" />
          <span>You're on the latest version.</span>
          <Button variant="ghost" size="sm" class="ml-2" @click="updateStatus = 'idle'">Dismiss</Button>
        </div>

        <!-- Error -->
        <div v-else-if="updateStatus === 'error'" class="space-y-2">
          <div class="flex items-center gap-2 text-sm text-destructive">
            <AlertCircle class="h-4 w-4" />
            <span>{{ updateError }}</span>
          </div>
          <Button variant="outline" size="sm" @click="checkForUpdates">
            <RefreshCw class="mr-2 h-4 w-4" />
            Retry
          </Button>
        </div>
      </CardContent>
    </Card>

    <!-- What's New -->
    <Card class="mb-6">
      <CardHeader>
        <CardTitle class="text-base">What's New</CardTitle>
        <CardDescription>See what changed in recent releases.</CardDescription>
      </CardHeader>
      <CardContent>
        <!-- Idle -->
        <Button v-if="changelogStatus === 'idle'" variant="outline" @click="fetchChangelog">
          <FileText class="mr-2 h-4 w-4" />
          View Changelog
        </Button>

        <!-- Loading -->
        <div v-else-if="changelogStatus === 'loading'" class="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" />
          Loading changelog...
        </div>

        <!-- Loaded -->
        <div v-else-if="changelogStatus === 'loaded'" class="space-y-3">
          <div v-if="changelogEntries.length === 0" class="text-sm text-muted-foreground">
            No changelog entries available.
          </div>
          <div
            v-for="entry in changelogEntries"
            :key="entry.version"
            class="rounded-lg border border-border p-3"
          >
            <div class="flex items-center gap-2">
              <span class="font-mono text-sm font-medium">{{ entry.version }}</span>
              <span class="text-xs text-muted-foreground">{{ entry.date }}</span>
            </div>
            <p class="mt-1 text-sm text-muted-foreground">{{ entry.description }}</p>
          </div>
        </div>

        <!-- Error -->
        <div v-else-if="changelogStatus === 'error'" class="space-y-2">
          <div class="flex items-center gap-2 text-sm text-destructive">
            <AlertCircle class="h-4 w-4" />
            <span>{{ changelogError }}</span>
          </div>
          <Button variant="outline" size="sm" @click="fetchChangelog">
            <RefreshCw class="mr-2 h-4 w-4" />
            Retry
          </Button>
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
