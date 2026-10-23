<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ fraction: number; on: boolean; held?: boolean; size?: number }>()
const bottom = 38
const top = 18.5
const level = computed(() => props.held || !props.on ? 1 : Math.max(0.12, Math.min(1, props.fraction)))
const surface = computed(() => bottom - (bottom - top) * level.value)
const flameBase = computed(() => surface.value - 1.8)
const flame = computed(() => {
  const b = flameBase.value
  return `M20 ${b - 14.5} C 22.6 ${b - 10.6} 24.6 ${b - 8.1} 24.6 ${b - 4.6} a4.6 4.6 0 0 1 -9.2 0 c0 -3.5 2 -6 4.6 -9.9 z`
})
</script>

<template>
  <svg class="status-candle" :class="{ on, held }" :width="size ?? 44" :height="size ?? 44" viewBox="0 0 40 40" aria-hidden="true">
    <rect v-if="!on && !held" class="candle-outline" x="12.75" y="19.25" width="14.5" height="18" rx="2.6"/>
    <path v-if="!on && !held" class="candle-wick" d="M20 15.5v3"/>
    <rect v-if="held" class="candle-outline is-held" x="12.75" y="19.25" width="14.5" height="18" rx="2.6"/>
    <rect v-if="on" class="candle-body" x="12" :y="surface" width="16" :height="bottom - surface" rx="2.8"/>
    <path v-if="on || held" class="candle-flame" :d="flame"/>
  </svg>
</template>
