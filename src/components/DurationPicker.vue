<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import type { StartRequest } from '../lib/model'
import { customMinutes, parseClock, presets } from '../lib/session'

defineProps<{ active: string | null; busy: boolean }>()
const emit = defineEmits<{ start: [request: StartRequest] }>()
const editor = ref<'until' | 'custom' | null>(null)
const untilValue = ref('')
const hours = ref(1)
const minutes = ref(30)
const untilInput = ref<HTMLInputElement | null>(null)
const hoursInput = ref<HTMLInputElement | null>(null)
const untilTime = computed(() => parseClock(untilValue.value))
const custom = computed(() => customMinutes(Number(hours.value), Number(minutes.value)))

function defaultUntil(): string {
  const date = new Date(Date.now() + 60 * 60_000)
  date.setMinutes(date.getMinutes() < 30 ? 30 : 60, 0, 0)
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`
}

async function open(kind: 'until' | 'custom'): Promise<void> {
  editor.value = editor.value === kind ? null : kind
  if (editor.value === 'until' && !untilValue.value) untilValue.value = defaultUntil()
  await nextTick()
  if (editor.value === 'until') untilInput.value?.focus()
  if (editor.value === 'custom') hoursInput.value?.select()
}

function choose(request: StartRequest): void {
  editor.value = null
  emit('start', request)
}

function submitUntil(): void {
  if (untilTime.value) choose({ mode: 'until', ...untilTime.value })
}

function submitCustom(): void {
  if (custom.value) choose({ mode: 'for', minutes: custom.value })
}
</script>

<template>
  <div class="duration-grid" role="group" aria-label="Keep awake for">
    <button v-for="preset in presets" :key="preset.id" class="duration-option" :class="{ selected: active === preset.id }" :aria-pressed="active === preset.id" :aria-keyshortcuts="String(presets.indexOf(preset) + 1)" :disabled="busy" @click="choose(preset.request)">{{ preset.label }}</button>
    <button class="duration-option" :class="{ selected: active === 'until', editing: editor === 'until' }" :aria-pressed="active === 'until'" :aria-expanded="editor === 'until'" aria-controls="duration-editor" :disabled="busy" @click="open('until')">Until…</button>
    <button class="duration-option" :class="{ selected: active === 'custom', editing: editor === 'custom' }" :aria-pressed="active === 'custom'" :aria-expanded="editor === 'custom'" aria-controls="duration-editor" :disabled="busy" @click="open('custom')">Custom…</button>
    <button class="duration-option" :class="{ selected: active === 'indefinite' }" :aria-pressed="active === 'indefinite'" aria-keyshortcuts="0" :disabled="busy" @click="choose({ mode: 'indefinite' })">Indefinitely</button>
  </div>
  <form v-if="editor === 'until'" id="duration-editor" class="duration-editor" @submit.prevent="submitUntil">
    <label for="until-time">Until</label>
    <input id="until-time" ref="untilInput" v-model="untilValue" class="text-input time-input" type="time" required :aria-invalid="!untilTime">
    <button class="button button-primary button-small" type="submit" :disabled="busy || !untilTime"><AppIcon name="check" :size="14"/>Start</button>
  </form>
  <form v-else-if="editor === 'custom'" id="duration-editor" class="duration-editor" @submit.prevent="submitCustom">
    <label class="sr-only" for="custom-hours">Hours</label>
    <input id="custom-hours" ref="hoursInput" v-model.number="hours" class="text-input number-input" type="number" min="0" max="168" inputmode="numeric" :aria-invalid="!custom">
    <span class="unit" aria-hidden="true">h</span>
    <label class="sr-only" for="custom-minutes">Minutes</label>
    <input id="custom-minutes" v-model.number="minutes" class="text-input number-input" type="number" min="0" max="59" step="5" inputmode="numeric" :aria-invalid="!custom">
    <span class="unit" aria-hidden="true">m</span>
    <button class="button button-primary button-small" type="submit" :disabled="busy || !custom"><AppIcon name="check" :size="14"/>Start</button>
  </form>
</template>
