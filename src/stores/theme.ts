import { ref } from "vue";
import { defineStore } from "pinia";

export type ThemeMode = "light" | "dark" | "system";

export const useThemeStore = defineStore("theme", () => {
  const mode = ref<ThemeMode>(
    (localStorage.getItem("opt-outta-theme") as ThemeMode) || "system"
  );

  function applyTheme() {
    const effective =
      mode.value === "system"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
          ? "dark"
          : "light"
        : mode.value;

    document.documentElement.classList.toggle("dark", effective === "dark");
  }

  function setMode(m: ThemeMode) {
    mode.value = m;
    localStorage.setItem("opt-outta-theme", m);
    applyTheme();
  }

  // Watch for system theme changes
  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  mql.addEventListener("change", () => {
    if (mode.value === "system") {
      applyTheme();
    }
  });

  return { mode, applyTheme, setMode };
});
