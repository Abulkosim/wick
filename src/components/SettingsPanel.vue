<script setup lang="ts">
import { computed } from 'vue'
import { version } from '../../package.json'
import type { Settings } from '../lib/model'
import { shortDuration } from '../lib/session'
import AppIcon from './AppIcon.vue'
import AppSelect from './ui/AppSelect.vue'

const props = defineProps<{ settings: Settings; recovered: string[]; recovery: string | null; busy: boolean; save: (settings: Settings) => Promise<boolean>; reset: () => Promise<boolean> }>()
defineEmits<{ back: [] }>()

const themes: { id: Settings['theme']; label: string; icon: 'system' | 'sun' | 'moon' }[] = [
  { id: 'system', label: 'System', icon: 'system' },
  { id: 'light', label: 'Light', icon: 'sun' },
  { id: 'dark', label: 'Dark', icon: 'moon' },
]
const startOptions = computed(() => {
  const minutes = [15, 60, 120, ...(props.settings.startMinutes && ![15, 60, 120].includes(props.settings.startMinutes) ? [props.settings.startMinutes] : [])].sort((a, b) => a - b)
  return [
    { value: 'indefinite', label: 'Until stopped' },
    ...minutes.map(value => ({ value: String(value), label: value === 15 ? '15 minutes' : value === 60 ? '1 hour' : value === 120 ? '2 hours' : shortDuration(value * 60_000) })),
  ]
})
const clickOptions: { value: Settings['iconClick']; label: string }[] = [
  { value: 'open', label: 'Shows Wick' },
  { value: 'toggle', label: 'Starts or stops' },
]

function patch(change: Partial<Settings>): void {
  void props.save({ ...props.settings, ...change })
}

function setStart(value: string): void {
  patch({ startMinutes: value === 'indefinite' ? null : Number(value) })
}

function setClick(value: string): void {
  if (value === 'open' || value === 'toggle') patch({ iconClick: value })
}
</script>

<template>
  <div class="settings-view">
    <header class="settings-header"><button class="icon-button back-button" aria-label="Back" @click="$emit('back')"><AppIcon name="back" :size="18"/></button><h1>Settings</h1><span class="settings-autosave">Saves automatically</span></header>
    <div class="settings-content">
      <div v-if="recovery" class="error-note recovery-note" role="alert"><AppIcon name="alert" :size="16"/><div><p><strong>Settings could not be read.</strong> Wick is using defaults and won’t save until you reset. {{ recovery }}</p><button class="button button-small" :disabled="busy" @click="reset">Back up and reset</button></div></div>
      <p v-else-if="recovered.length" class="recovered-note" role="status"><AppIcon name="alert" :size="13"/><span>{{ recovered.join(' ') }} A backup is kept next to the settings file.</span></p>
      <section class="settings-section">
        <h2>General</h2>
        <div class="setting-row">
          <div><label for="start-duration">Start keeps awake</label><p>What Start begins.</p></div>
          <AppSelect id="start-duration" :model-value="settings.startMinutes === null ? 'indefinite' : String(settings.startMinutes)" :options="startOptions" :disabled="busy" @update:model-value="setStart"/>
        </div>
        <div class="setting-row">
          <div><label for="icon-click">Clicking the icon</label><p>Right-click shows the menu.</p></div>
          <AppSelect id="icon-click" :model-value="settings.iconClick" :options="clickOptions" :disabled="busy" @update:model-value="setClick"/>
        </div>
      </section>
      <section class="settings-section">
        <h2>Appearance</h2>
        <div class="theme-options"><button v-for="theme in themes" :key="theme.id" class="theme-option" :class="{ selected: settings.theme === theme.id }" :aria-pressed="settings.theme === theme.id" :disabled="busy" @click="patch({ theme: theme.id })"><span class="theme-thumbnail" :class="`thumbnail-${theme.id}`"><span/><span/><span/></span><span><AppIcon :name="theme.icon" :size="13"/>{{ theme.label }}</span><span v-if="settings.theme === theme.id" class="theme-check"><AppIcon name="check" :size="10"/></span></button></div>
      </section>
      <p class="settings-footnote">Version {{ version }}</p>
    </div>
  </div>
</template>
