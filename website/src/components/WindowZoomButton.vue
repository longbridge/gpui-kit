<script setup lang="ts">
import { Maximize, Minimize } from "lucide-vue-next";

// The traffic lights alone are too quiet to advertise that a frame can be
// zoomed, so every live window carries a real control at the end of its title
// bar. It stays hidden until the pointer reaches the bar, the way macOS
// toolbars reveal their own accessory buttons.
defineProps<{ zoomed: boolean; label: string }>();
</script>

<template>
    <button
        type="button"
        class="mac-window__action"
        :title="label"
        :aria-label="label"
        :aria-pressed="zoomed"
    >
        <Minimize v-if="zoomed" :size="14" />
        <Maximize v-else :size="14" />
    </button>
</template>
