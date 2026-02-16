<script setup lang="ts">
import { useRoute } from "vue-router";
import { LayoutDashboard, UserRound, Shield, Database, Clock, Settings } from "lucide-vue-next";

const route = useRoute();

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/profile", label: "Profile", icon: UserRound },
  { path: "/brokers", label: "Brokers", icon: Database },
  { path: "/history", label: "History", icon: Clock },
  { path: "/settings", label: "Settings", icon: Settings },
];

function isActive(path: string): boolean {
  return route.path === path;
}
</script>

<template>
  <aside class="flex w-56 flex-col border-r border-sidebar-border bg-sidebar">
    <div class="flex h-14 items-center gap-2 border-b border-sidebar-border px-5">
      <Shield class="h-5 w-5 text-sidebar-foreground" />
      <h1 class="text-lg font-bold tracking-tight text-sidebar-foreground">Opt-Outta</h1>
    </div>
    <nav class="flex-1 space-y-1 px-3 py-4">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors"
        :class="
          isActive(item.path)
            ? 'bg-sidebar-accent text-sidebar-accent-foreground'
            : 'text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
        "
      >
        <component :is="item.icon" class="h-4 w-4" />
        {{ item.label }}
      </router-link>
    </nav>
    <div class="border-t border-sidebar-border px-5 py-3">
      <p class="text-xs text-muted-foreground">v0.1.0</p>
    </div>
  </aside>
</template>
