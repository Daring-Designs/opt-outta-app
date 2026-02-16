<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useProfileStore } from "../stores/profile";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";

const store = useProfileStore();
const saving = ref(false);
const saved = ref(false);

onMounted(async () => {
  if (!store.loaded) await store.loadProfile();
});

function addAlternateEmail() {
  store.profile.alternateEmails.push("");
}

function removeAlternateEmail(index: number) {
  store.profile.alternateEmails.splice(index, 1);
}

function addAlternatePhone() {
  store.profile.alternatePhones.push("");
}

function removeAlternatePhone(index: number) {
  store.profile.alternatePhones.splice(index, 1);
}

function addPreviousAddress() {
  store.profile.previousAddresses.push({ address: "", city: "", state: "", zip: "" });
}

function removePreviousAddress(index: number) {
  store.profile.previousAddresses.splice(index, 1);
}

async function save() {
  saving.value = true;
  saved.value = false;
  try {
    await store.saveProfile();
    saved.value = true;
    setTimeout(() => (saved.value = false), 3000);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="mx-auto max-w-2xl">
    <h1 class="mb-6 text-2xl font-bold">Profile</h1>
    <p class="mb-6 text-sm text-muted-foreground">
      Your personal information is encrypted and stored locally. It's only used to fill opt-out
      forms.
    </p>

    <form @submit.prevent="save" class="space-y-8">
      <!-- Required Fields -->
      <section>
        <h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          Required
        </h2>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <Label class="mb-1 block">First Name</Label>
            <Input v-model="store.profile.firstName" type="text" />
          </div>
          <div>
            <Label class="mb-1 block">Last Name</Label>
            <Input v-model="store.profile.lastName" type="text" />
          </div>
        </div>
        <div class="mt-4">
          <Label class="mb-1 block">Email</Label>
          <Input v-model="store.profile.email" type="email" />
        </div>
      </section>

      <!-- Recommended Fields -->
      <section>
        <h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          Recommended
        </h2>
        <div class="space-y-4">
          <div>
            <Label class="mb-1 block">Phone</Label>
            <Input v-model="store.profile.phone" type="tel" />
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div class="col-span-2">
              <Label class="mb-1 block">Address</Label>
              <Input v-model="store.profile.address" type="text" />
            </div>
            <div>
              <Label class="mb-1 block">City</Label>
              <Input v-model="store.profile.city" type="text" />
            </div>
            <div class="grid grid-cols-2 gap-4">
              <div>
                <Label class="mb-1 block">State</Label>
                <Input v-model="store.profile.state" type="text" />
              </div>
              <div>
                <Label class="mb-1 block">ZIP</Label>
                <Input v-model="store.profile.zip" type="text" />
              </div>
            </div>
          </div>
          <div>
            <Label class="mb-1 block">Date of Birth</Label>
            <Input v-model="store.profile.dob" type="date" />
          </div>
        </div>
      </section>

      <!-- Optional Fields -->
      <section>
        <h2 class="mb-4 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          Optional
        </h2>

        <!-- Alternate Emails -->
        <div class="mb-6">
          <div class="mb-2 flex items-center justify-between">
            <Label>Alternate Emails</Label>
            <button
              type="button"
              class="text-sm text-primary hover:text-primary/80"
              @click="addAlternateEmail"
            >
              + Add
            </button>
          </div>
          <div
            v-for="(_, index) in store.profile.alternateEmails"
            :key="'ae-' + index"
            class="mb-2 flex gap-2"
          >
            <Input
              v-model="store.profile.alternateEmails[index]"
              type="email"
              class="flex-1"
            />
            <button
              type="button"
              class="px-2 text-sm text-destructive hover:text-destructive/80"
              @click="removeAlternateEmail(index)"
            >
              Remove
            </button>
          </div>
        </div>

        <!-- Alternate Phones -->
        <div class="mb-6">
          <div class="mb-2 flex items-center justify-between">
            <Label>Alternate Phones</Label>
            <button
              type="button"
              class="text-sm text-primary hover:text-primary/80"
              @click="addAlternatePhone"
            >
              + Add
            </button>
          </div>
          <div
            v-for="(_, index) in store.profile.alternatePhones"
            :key="'ap-' + index"
            class="mb-2 flex gap-2"
          >
            <Input
              v-model="store.profile.alternatePhones[index]"
              type="tel"
              class="flex-1"
            />
            <button
              type="button"
              class="px-2 text-sm text-destructive hover:text-destructive/80"
              @click="removeAlternatePhone(index)"
            >
              Remove
            </button>
          </div>
        </div>

        <!-- Previous Addresses -->
        <div>
          <div class="mb-2 flex items-center justify-between">
            <Label>Previous Addresses</Label>
            <button
              type="button"
              class="text-sm text-primary hover:text-primary/80"
              @click="addPreviousAddress"
            >
              + Add
            </button>
          </div>
          <div
            v-for="(addr, index) in store.profile.previousAddresses"
            :key="'pa-' + index"
            class="mb-3 rounded-lg border border-border p-3"
          >
            <div class="mb-2 flex justify-end">
              <button
                type="button"
                class="text-sm text-destructive hover:text-destructive/80"
                @click="removePreviousAddress(index)"
              >
                Remove
              </button>
            </div>
            <div class="space-y-2">
              <Input v-model="addr.address" type="text" placeholder="Address" />
              <div class="grid grid-cols-3 gap-2">
                <Input v-model="addr.city" type="text" placeholder="City" />
                <Input v-model="addr.state" type="text" placeholder="State" />
                <Input v-model="addr.zip" type="text" placeholder="ZIP" />
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Save -->
      <div class="flex items-center gap-3">
        <Button type="submit" :disabled="saving">
          {{ saving ? "Saving..." : "Save Profile" }}
        </Button>
        <span v-if="saved" class="text-sm text-green-600">Saved!</span>
      </div>
    </form>
  </div>
</template>
