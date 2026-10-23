<script setup lang="ts">
import AppIcon from './AppIcon.vue'
import AppTooltip from './ui/AppTooltip.vue'
import type { Other } from '../lib/model'
import { otherDetail } from '../lib/others'

defineProps<{ others: Other[]; also: boolean }>()
</script>

<template>
  <section class="card others-card" aria-labelledby="others-label">
    <h2 id="others-label" class="card-label">{{ also ? 'Also keeping your Mac awake' : 'Keeping your Mac awake' }}</h2>
    <ul class="trigger-list">
      <li v-for="other in others" :key="other.pid" class="trigger-row other-row">
        <span class="trigger-mark"><AppIcon :name="other.parent || other.name === 'caffeinate' ? 'terminal' : 'app'" :size="15"/></span>
        <span class="trigger-copy">
          <AppTooltip :text="`${other.name} · process ${other.pid}`"><span class="trigger-sentence" tabindex="0"><strong>{{ other.name }}</strong></span></AppTooltip>
          <span class="trigger-status">{{ otherDetail(other) }}</span>
        </span>
      </li>
    </ul>
  </section>
</template>
