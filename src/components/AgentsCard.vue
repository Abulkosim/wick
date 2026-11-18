<script setup lang="ts">
import { computed, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import type { Agent } from '../lib/model'
import { agentPlace, agentStatus } from '../lib/agents'
import { device } from '../lib/platform'

const props = defineProps<{ agents: Agent[]; now: number; enabled: boolean; busy: boolean }>()
defineEmits<{ enable: [] }>()
const limit = 4
const expanded = ref(false)
const shown = computed(() => expanded.value || props.agents.length <= limit ? props.agents : props.agents.slice(0, limit - 1))
const hidden = computed(() => props.agents.length - shown.value.length)
const folds = computed(() => props.agents.length > limit)
</script>

<template>
  <section class="card agents-card" :class="{ 'is-off': !enabled }" aria-labelledby="agents-label">
    <h2 id="agents-label" class="card-label">Agents</h2>
    <ul id="agents-list" class="trigger-list">
      <li v-for="agent in shown" :key="agent.id" class="trigger-row agent-row" :class="{ running: enabled && agent.state === 'working' && !agent.paused, waiting: agent.state === 'waiting' || agent.paused }">
        <span class="trigger-mark"><AppIcon name="spark" :size="15"/></span>
        <span class="trigger-copy">
          <span class="trigger-sentence"><strong>{{ agent.name }}</strong><template v-if="agentPlace(agent)"><span class="agent-separator"> · </span>{{ agentPlace(agent) }}</template></span>
          <span class="trigger-status"><span class="status-dot"/>{{ agentStatus(agent, now) }}</span>
        </span>
      </li>
    </ul>
    <button v-if="folds" class="text-button agents-more" aria-controls="agents-list" :aria-expanded="expanded" @click="expanded = !expanded">{{ expanded ? 'Show fewer' : `${hidden} more` }}</button>
    <p v-if="!enabled" class="agents-off-note"><span>Agents don’t keep your {{ device }} awake.</span><button class="text-button inline-action" :disabled="busy" @click="$emit('enable')">Turn on</button></p>
  </section>
</template>
