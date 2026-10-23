<script setup lang="ts">
import { computed } from 'vue'
import { version } from '../../package.json'
import type { Power, Settings } from '../lib/model'
import { hasBattery } from '../lib/power'
import { shortDuration } from '../lib/session'
import AppIcon from './AppIcon.vue'
import AppSelect from './ui/AppSelect.vue'
import AppSwitch from './ui/AppSwitch.vue'
import ShortcutField from './ShortcutField.vue'
import UpdateSettings from './settings/UpdateSettings.vue'
import type { UpdateState } from '../lib/updater'

const props = defineProps<{ updateState: UpdateState; updateBusy: boolean; demo: boolean; settings: Settings; power: Power; recovered: string[]; recovery: string | null; busy: boolean; save: (settings: Settings) => Promise<boolean>; reset: () => Promise<boolean> }>()
defineEmits<{ back: []; checkUpdate: []; installUpdate: [] }>()

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
const thresholds = [10, 20, 30, 50]
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
    <header class="settings-header"><button class="icon-button back-button" aria-label="Back to overview" @click="$emit('back')"><AppIcon name="back" :size="18"/></button><h1>Settings</h1><span class="settings-autosave">Saves automatically</span></header>
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
        <div class="setting-row">
          <div><span id="shortcut-label" class="setting-label">Shortcut</span><p>Starts or stops anywhere.</p></div>
          <ShortcutField :model-value="settings.shortcut" labelledby="shortcut-label" :disabled="busy" @update:model-value="patch({ shortcut: $event })"/>
        </div>
        <div class="setting-row"><div><span id="time-left-label" class="setting-label">Show time left in the menu bar</span><p>Beside the candle, for timed sessions.</p></div><AppSwitch :model-value="settings.showTimeLeft" aria-labelledby="time-left-label" :disabled="busy" @update:model-value="patch({ showTimeLeft: $event })"/></div>
        <div class="setting-row"><div><span id="lock-label" class="setting-label">Pause while the screen is locked</span><p>Lets the Mac sleep until you unlock it.</p></div><AppSwitch :model-value="settings.pauseWhenLocked" aria-labelledby="lock-label" :disabled="busy" @update:model-value="patch({ pauseWhenLocked: $event })"/></div>
        <div class="setting-row"><span id="login-label" class="setting-label">Launch at login</span><AppSwitch :model-value="settings.launchAtLogin" aria-labelledby="login-label" :disabled="busy" @update:model-value="patch({ launchAtLogin: $event })"/></div>
      </section>
      <section v-if="hasBattery(power)" class="settings-section">
        <h2>Battery</h2>
        <div class="setting-row"><div><span id="battery-guard-label" class="setting-label">Stop on low battery</span><p>Only while on battery power.</p></div><AppSwitch :model-value="settings.batteryGuard" aria-labelledby="battery-guard-label" :disabled="busy" @update:model-value="patch({ batteryGuard: $event })"/></div>
        <div class="threshold-options" :class="{ 'is-disabled': !settings.batteryGuard }" role="group" aria-label="Stop below"><span>Stop below</span><button v-for="threshold in thresholds" :key="threshold" class="threshold-chip" :class="{ selected: settings.batteryThreshold === threshold }" :aria-pressed="settings.batteryThreshold === threshold" :disabled="busy || !settings.batteryGuard" @click="patch({ batteryThreshold: threshold })">{{ threshold }}%</button></div>
        <div class="setting-row"><div><span id="low-power-label" class="setting-label">Stop in Low Power Mode</span><p>When it turns on while on battery.</p></div><AppSwitch :model-value="settings.stopInLowPowerMode" aria-labelledby="low-power-label" :disabled="busy" @update:model-value="patch({ stopInLowPowerMode: $event })"/></div>
      </section>
      <section class="settings-section">
        <h2>Notifications</h2>
        <div class="setting-row"><div><span id="notify-end-label" class="setting-label">When a session ends</span><p>Also when the battery stops it.</p></div><AppSwitch :model-value="settings.notifyOnEnd" aria-labelledby="notify-end-label" :disabled="busy" @update:model-value="patch({ notifyOnEnd: $event })"/></div>
      </section>
      <section class="settings-section">
        <h2>Appearance</h2>
        <div class="theme-options"><button v-for="theme in themes" :key="theme.id" class="theme-option" :class="{ selected: settings.theme === theme.id }" :aria-pressed="settings.theme === theme.id" :disabled="busy" @click="patch({ theme: theme.id })"><span class="theme-thumbnail" :class="`thumbnail-${theme.id}`"><span/><span/><span/></span><span><AppIcon :name="theme.icon" :size="13"/>{{ theme.label }}</span><span v-if="settings.theme === theme.id" class="theme-check"><AppIcon name="check" :size="10"/></span></button></div>
      </section>
      <UpdateSettings :state="updateState" :busy="updateBusy" :demo="demo" :auto-check="settings.autoCheckUpdates" :settings-busy="busy" @check="$emit('checkUpdate')" @install="$emit('installUpdate')" @auto-check="patch({ autoCheckUpdates: $event })"/>
      <p class="settings-footnote">Version {{ version }}</p>
    </div>
  </div>
</template>
