<script setup lang="ts">
import { computed } from "vue";
import { BrokerStatus } from "../types";

const props = defineProps<{ status: BrokerStatus }>();

const config = computed(() => {
  switch (props.status) {
    case BrokerStatus.NotStarted:
      return { label: "Not Started", classes: "bg-muted text-muted-foreground" };
    case BrokerStatus.Submitted:
      return { label: "Submitted", classes: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400" };
    case BrokerStatus.PendingVerification:
      return { label: "Pending", classes: "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400" };
    case BrokerStatus.Confirmed:
      return { label: "Confirmed", classes: "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" };
    case BrokerStatus.ReListed:
      return { label: "Re-listed", classes: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400" };
    case BrokerStatus.Failed:
      return { label: "Failed", classes: "bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400" };
    default:
      return { label: "Unknown", classes: "bg-muted text-muted-foreground" };
  }
});
</script>

<template>
  <span
    class="inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium"
    :class="config.classes"
  >
    {{ config.label }}
  </span>
</template>
