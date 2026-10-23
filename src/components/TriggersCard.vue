<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import AppSelect from './ui/AppSelect.vue'
import AppSwitch from './ui/AppSwitch.vue'
import AppTooltip from './ui/AppTooltip.vue'
import type { RunningApp, Trigger } from '../lib/model'
import { dayLetters, daysLabel, processSuggestions, triggerId, validProcessName, validSchedule } from '../lib/triggers'

const props = defineProps<{ triggers: Trigger[]; running: string[]; busy: boolean; save: (triggers: Trigger[]) => Promise<boolean>; loadApps: () => Promise<RunningApp[]> }>()
const adding = ref(false)
const kind = ref<'app' | 'process' | 'condition'>('app')
const apps = ref<RunningApp[]>([])
const appPath = ref('')
const processName = ref('')
const processInput = ref<HTMLInputElement | null>(null)
const addButton = ref<HTMLButtonElement | null>(null)
const appOptions = computed(() => apps.value.filter(app => !props.triggers.some(trigger => trigger.path === app.path)).map(app => ({ value: app.path, label: app.name })))
const suggestions = computed(() => processSuggestions.filter(name => !props.triggers.some(trigger => trigger.kind === 'process' && trigger.name === name)))
const nameError = computed(() => processName.value.trim() && !validProcessName(processName.value) ? 'Type a command name, like ffmpeg.' : null)
const condition = ref<'power' | 'display' | 'schedule' | null>(null)
const conditions = computed(() => ([
  { kind: 'power', name: 'Power adapter', label: 'On power' },
  { kind: 'display', name: 'External display', label: 'External display connected' },
  { kind: 'schedule', name: 'Schedule', label: 'Schedule' },
] as const).filter(option => option.kind === 'schedule' || !props.triggers.some(trigger => trigger.kind === option.kind)))
const days = ref<number[]>([0, 1, 2, 3, 4])
const start = ref('09:00')
const end = ref('18:00')
const scheduleError = computed(() => validSchedule(days.value, start.value, end.value))
const canAdd = computed(() => kind.value === 'app' ? Boolean(appPath.value) : kind.value === 'process' ? validProcessName(processName.value) : condition.value === 'schedule' ? !scheduleError.value : condition.value !== null)

function toggleDay(day: number): void {
  days.value = days.value.includes(day) ? days.value.filter(value => value !== day) : [...days.value, day].sort((a, b) => a - b)
}

function status(trigger: Trigger): string {
  if (!trigger.enabled) return 'Off'
  const now = props.running.includes(trigger.id)
  if (trigger.kind === 'schedule') return now ? 'On now' : 'Off now'
  if (trigger.kind === 'power') return now ? 'Plugged in' : 'On battery'
  if (trigger.kind === 'display') return now ? 'Connected' : 'Not connected'
  return now ? 'Running' : 'Not running'
}

function sentence(trigger: Trigger): { strong: string; rest: string; lead?: string } {
  if (trigger.kind === 'schedule' && trigger.schedule) return { strong: daysLabel(trigger.schedule.days), rest: ` ${trigger.schedule.start}–${trigger.schedule.end}` }
  if (trigger.kind === 'power') return { lead: '', strong: 'On power', rest: '' }
  if (trigger.kind === 'display') return { strong: 'External display', rest: ' connected' }
  return { strong: trigger.name, rest: ' is running' }
}

function label(trigger: Trigger): string {
  if (trigger.kind === 'schedule') return `Stay awake ${trigger.name}`
  if (trigger.kind === 'power') return 'Stay awake while on power'
  if (trigger.kind === 'display') return 'Stay awake while an external display is connected'
  return `Stay awake while ${trigger.name} is running`
}

async function open(): Promise<void> {
  adding.value = true
  appPath.value = ''
  processName.value = ''
  condition.value = null
  apps.value = await props.loadApps()
  if (!apps.value.length) kind.value = 'process'
}

async function choose(next: 'app' | 'process' | 'condition'): Promise<void> {
  kind.value = next
  if (next === 'condition') condition.value = conditions.value[0]?.kind ?? null
  if (next === 'process') { await nextTick(); processInput.value?.focus() }
}

async function close(): Promise<void> {
  adding.value = false
  await nextTick()
  addButton.value?.focus()
}

async function add(): Promise<void> {
  if (!canAdd.value) return
  const app = apps.value.find(item => item.path === appPath.value)
  const picked = conditions.value.find(option => option.kind === condition.value)
  const schedule = { days: days.value, start: start.value, end: end.value }
  const trigger: Trigger = kind.value === 'condition' && picked?.kind === 'schedule'
    ? { id: triggerId(), kind: 'schedule', name: `${daysLabel(schedule.days)} ${schedule.start}–${schedule.end}`, schedule, enabled: true }
    : kind.value === 'condition' && picked
    ? { id: triggerId(), kind: picked.kind, name: picked.name, enabled: true }
    : kind.value === 'app' && app
      ? { id: triggerId(), kind: 'app', name: app.name, path: app.path, enabled: true }
      : { id: triggerId(), kind: 'process', name: processName.value.trim(), enabled: true }
  if (await props.save([...props.triggers, trigger])) await close()
}

function update(id: string, change: Partial<Trigger>): void {
  void props.save(props.triggers.map(trigger => trigger.id === id ? { ...trigger, ...change } : trigger))
}

function remove(id: string): void {
  void props.save(props.triggers.filter(trigger => trigger.id !== id))
}
</script>

<template>
  <section class="card" aria-labelledby="triggers-label">
    <div class="card-heading">
      <h2 id="triggers-label" class="card-label">Stay awake while</h2>
      <button v-if="!adding" ref="addButton" class="text-button card-action" :disabled="busy" @click="open"><AppIcon name="plus" :size="13"/>Add</button>
    </div>
    <p v-if="!triggers.length && !adding" class="card-empty">An app or command is running, like Xcode or claude.</p>
    <ul v-if="triggers.length" class="trigger-list">
      <li v-for="trigger in triggers" :key="trigger.id" class="trigger-row" :class="{ running: status(trigger) === 'Running', disabled: !trigger.enabled }">
        <span class="trigger-mark"><AppIcon :name="trigger.kind === 'app' ? 'app' : trigger.kind === 'power' ? 'bolt' : trigger.kind === 'display' ? 'display' : trigger.kind === 'schedule' ? 'clock' : 'terminal'" :size="15"/></span>
        <span class="trigger-copy">
          <span class="trigger-sentence">{{ sentence(trigger).lead }}<strong>{{ sentence(trigger).strong }}</strong>{{ sentence(trigger).rest }}</span>
          <span class="trigger-status"><span class="status-dot"/>{{ status(trigger) }}</span>
        </span>
        <AppTooltip text="Remove"><button class="icon-button trigger-remove" :aria-label="`Remove ${trigger.kind === 'power' || trigger.kind === 'display' ? trigger.name.toLowerCase() : trigger.kind === 'schedule' ? `schedule ${trigger.name}` : trigger.name}`" :disabled="busy" @click="remove(trigger.id)"><AppIcon name="close" :size="13"/></button></AppTooltip>
        <AppSwitch :model-value="trigger.enabled" :aria-label="label(trigger)" :disabled="busy" @update:model-value="update(trigger.id, { enabled: $event })"/>
      </li>
    </ul>
    <form v-if="adding" class="trigger-form" @submit.prevent="add" @keydown.esc.stop.prevent="close">
      <div class="segmented" role="radiogroup" aria-label="Trigger type">
        <button type="button" role="radio" :aria-checked="kind === 'app'" :class="{ selected: kind === 'app' }" @click="choose('app')"><AppIcon name="app" :size="14"/>App</button>
        <button type="button" role="radio" :aria-checked="kind === 'process'" :class="{ selected: kind === 'process' }" @click="choose('process')"><AppIcon name="terminal" :size="14"/>Command</button>
        <button type="button" role="radio" :aria-checked="kind === 'condition'" :class="{ selected: kind === 'condition' }" @click="choose('condition')"><AppIcon name="bolt" :size="14"/>Condition</button>
      </div>
      <template v-if="kind === 'app'">
        <AppSelect v-if="appOptions.length" v-model="appPath" class="app-picker" label="Running app" placeholder="Choose a running app" :options="appOptions"/>
        <p v-else class="card-empty">No other apps are running. Open the app first, or add its command.</p>
      </template>
      <template v-else-if="kind === 'condition'">
        <div v-if="conditions.length" class="condition-options" role="radiogroup" aria-label="Condition">
          <button v-for="option in conditions" :key="option.kind" type="button" role="radio" class="duration-option" :class="{ selected: condition === option.kind }" :aria-checked="condition === option.kind" @click="condition = option.kind">{{ option.label }}</button>
        </div>
        <div v-if="condition === 'schedule'" class="schedule-editor">
          <div class="day-chips" role="group" aria-label="Days">
            <button v-for="(letter, day) in dayLetters" :key="day" type="button" class="threshold-chip day-chip" :class="{ selected: days.includes(day) }" :aria-pressed="days.includes(day)" :aria-label="['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'][day]" @click="toggleDay(day)">{{ letter }}</button>
          </div>
          <div class="schedule-times">
            <label for="schedule-start">From</label>
            <input id="schedule-start" v-model="start" class="text-input time-input" type="time" required>
            <label for="schedule-end">to</label>
            <input id="schedule-end" v-model="end" class="text-input time-input" type="time" required>
          </div>
          <p v-if="scheduleError" class="field-error" role="alert">{{ scheduleError }}</p>
          <p v-else-if="end <= start" class="card-empty">Runs overnight into the next day.</p>
        </div>
      </template>
      <template v-else>
        <label class="sr-only" for="process-name">Command name</label>
        <input id="process-name" ref="processInput" v-model="processName" class="text-input process-input" placeholder="ffmpeg" autocomplete="off" spellcheck="false" autocapitalize="off" maxlength="64" :aria-invalid="Boolean(nameError)" aria-describedby="process-help">
        <p v-if="nameError" class="field-error" role="alert">{{ nameError }}</p>
        <div id="process-help" class="suggestions"><span>Try</span><button v-for="name in suggestions" :key="name" type="button" class="threshold-chip" @click="processName = name">{{ name }}</button></div>
      </template>
      <div class="form-actions">
        <button type="button" class="button button-small" @click="close">Cancel</button>
        <button type="submit" class="button button-primary button-small" :disabled="busy || !canAdd">Add</button>
      </div>
    </form>
  </section>
</template>
