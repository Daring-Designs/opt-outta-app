import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Profile } from "../types";

function emptyProfile(): Profile {
  return {
    firstName: "",
    lastName: "",
    email: "",
    phone: "",
    address: "",
    city: "",
    state: "",
    zip: "",
    dob: "",
    alternateEmails: [],
    alternatePhones: [],
    previousAddresses: [],
  };
}

export const useProfileStore = defineStore("profile", () => {
  const profile = ref<Profile>(emptyProfile());
  const loaded = ref(false);
  const loading = ref(false);

  const completeness = computed(() => {
    const fields = [
      profile.value.firstName,
      profile.value.lastName,
      profile.value.email,
      profile.value.phone,
      profile.value.address,
      profile.value.city,
      profile.value.state,
      profile.value.zip,
      profile.value.dob,
    ];
    const filled = fields.filter((f) => f.trim().length > 0).length;
    return Math.round((filled / fields.length) * 100);
  });

  async function loadProfile() {
    loading.value = true;
    try {
      const data = await invoke<Profile | null>("get_profile");
      if (data) {
        profile.value = data;
      }
      loaded.value = true;
    } catch {
      loaded.value = true;
    } finally {
      loading.value = false;
    }
  }

  async function saveProfile() {
    await invoke("save_profile", { profile: profile.value });
  }

  async function deleteProfile() {
    await invoke("delete_profile");
    profile.value = emptyProfile();
  }

  return { profile, loaded, loading, completeness, loadProfile, saveProfile, deleteProfile };
});
