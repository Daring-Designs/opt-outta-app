import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Broker, BrokerRegistry } from "../types";

export const useBrokersStore = defineStore("brokers", () => {
  const brokers = ref<Broker[]>([]);
  const version = ref("");
  const loading = ref(false);
  const searchQuery = ref("");
  const categoryFilter = ref<string | null>(null);

  const filteredBrokers = computed(() => {
    let result = brokers.value;
    if (categoryFilter.value) {
      result = result.filter((b) => b.category === categoryFilter.value);
    }
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      result = result.filter((b) => b.name.toLowerCase().includes(q));
    }
    return result;
  });

  const categories = computed(() => {
    const cats = new Set(brokers.value.map((b) => b.category));
    return Array.from(cats).sort();
  });

  async function loadBrokers() {
    loading.value = true;
    try {
      const registry = await invoke<BrokerRegistry>("get_brokers");
      brokers.value = registry.brokers;
      version.value = registry.version;
    } catch (e) {
      console.error("Failed to load brokers:", e);
    } finally {
      loading.value = false;
    }
    // Fire-and-forget sync with remote registry
    syncRegistry();
  }

  async function syncRegistry() {
    try {
      const updated = await invoke<boolean>("sync_registry");
      if (updated) {
        const registry = await invoke<BrokerRegistry>("get_brokers");
        brokers.value = registry.brokers;
        version.value = registry.version;
      }
    } catch (e) {
      console.error("Registry sync failed:", e);
    }
  }

  return {
    brokers,
    version,
    loading,
    searchQuery,
    categoryFilter,
    filteredBrokers,
    categories,
    loadBrokers,
    syncRegistry,
  };
});
