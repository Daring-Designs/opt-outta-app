import { createRouter, createWebHistory } from "vue-router";
import DashboardView from "../views/DashboardView.vue";
import ProfileView from "../views/ProfileView.vue";
import SettingsView from "../views/SettingsView.vue";
import HistoryView from "../views/HistoryView.vue";
import PlaybooksView from "../views/PlaybooksView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "dashboard", component: DashboardView },
    { path: "/profile", name: "profile", component: ProfileView },
    { path: "/brokers", name: "brokers", component: PlaybooksView },
    { path: "/settings", name: "settings", component: SettingsView },
    { path: "/history", name: "history", component: HistoryView },
  ],
});

export default router;
