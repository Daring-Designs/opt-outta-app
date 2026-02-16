<script setup lang="ts">
import { onMounted } from "vue";
import AppSidebar from "./components/AppSidebar.vue";
import { useThemeStore } from "./stores/theme";

const themeStore = useThemeStore();

onMounted(() => {
  themeStore.applyTheme();
});
</script>

<template>
  <div class="flex h-screen bg-background text-foreground">
    <AppSidebar />
    <main class="flex-1 overflow-y-auto p-8">
      <router-view />
    </main>
  </div>
</template>
