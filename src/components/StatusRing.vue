<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ fraction: number; on: boolean; held?: boolean; size?: number }>()
const radius = 15
const circumference = 2 * Math.PI * radius
const offset = computed(() => circumference * (1 - (props.on ? Math.min(1, Math.max(0, props.fraction)) : 0)))
</script>

<template>
  <svg class="status-ring" :class="{ on }" :width="size ?? 44" :height="size ?? 44" viewBox="0 0 40 40" aria-hidden="true">
    <circle class="status-ring-track" cx="20" cy="20" :r="radius"/>
    <circle class="status-ring-arc" cx="20" cy="20" :r="radius" :stroke-dasharray="circumference" :stroke-dashoffset="offset" transform="rotate(-90 20 20)"/>
    <circle v-if="held" class="status-ring-dot" cx="20" cy="20" r="4.5"/>
  </svg>
</template>
