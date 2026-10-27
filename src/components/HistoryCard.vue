<script setup lang="ts">
import { computed } from 'vue'
import AppTooltip from './ui/AppTooltip.vue'
import type { Day } from '../lib/model'
import { bars, minutesText } from '../lib/history'

const props = defineProps<{ days: Day[] }>()
const today = computed(() => minutesText(props.days.at(-1)?.minutes ?? 0))
const list = computed(() => bars(props.days))
</script>

<template>
  <section class="card history-card" aria-labelledby="history-label">
    <div class="history-summary">
      <h2 id="history-label" class="card-label">Awake today</h2>
      <p class="history-today">{{ today }}</p>
    </div>
    <ol class="history-bars" aria-label="Last 7 days">
      <AppTooltip v-for="bar in list" :key="bar.date" :text="`${bar.label} · ${bar.duration}`">
        <li class="history-day" :class="{ today: bar.today }">
          <span class="history-track"><span class="history-bar" :style="{ height: `${bar.height * 100}%` }"/></span>
          <span class="history-initial" aria-hidden="true">{{ bar.initial }}</span>
          <span class="sr-only">{{ bar.label }}: {{ bar.duration }}</span>
        </li>
      </AppTooltip>
    </ol>
  </section>
</template>
