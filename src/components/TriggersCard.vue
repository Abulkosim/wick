<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import AppIcon from './AppIcon.vue'
import AppSelect from './ui/AppSelect.vue'
import AppSwitch from './ui/AppSwitch.vue'
import AppTooltip from './ui/AppTooltip.vue'
import type { RunningApp, Trigger } from '../lib/model'
import { processSuggestions, triggerId, validProcessName } from '../lib/triggers'

const props = defineProps<{ triggers: Trigger[]; running: string[]; busy: boolean; save: (triggers: Trigger[]) => Promise<boolean>; loadApps: () => Promise<RunningApp[]> }>()
const adding = ref(false)
const kind = ref<'app' | 'process'>('app')
const apps = ref<RunningApp[]>([])
const appPath = ref('')
const processName = ref('')
const processInput = ref<HTMLInputElement | null>(null)
const addButton = ref<HTMLButtonElement | null>(null)
const appOptions = computed(() => apps.value.filter(app => !props.triggers.some(trigger => trigger.path === app.path)).map(app => ({ value: app.path, label: app.name })))
const suggestions = computed(() => processSuggestions.filter(name => !props.triggers.some(trigger => trigger.kind === 'process' && trigger.name === name)))
const nameError = computed(() => processName.value.trim() && !validProcessName(processName.value) ? 'Type a command name, like ffmpeg.' : null)
const canAdd = computed(() => kind.value === 'app' ? Boolean(appPath.value) : validProcessName(processName.value))

function status(trigger: Trigger): string {
  if (!trigger.enabled) return 'Off'
  return props.running.includes(trigger.id) ? 'Running' : 'Not running'
}

async function open(): Promise<void> {
  adding.value = true
  appPath.value = ''
  processName.value = ''
  apps.value = await props.loadApps()
  if (!apps.value.length) kind.value = 'process'
}

async function choose(next: 'app' | 'process'): Promise<void> {
  kind.value = next
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
  const trigger: Trigger = kind.value === 'app' && app
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
        <span class="trigger-mark"><AppIcon :name="trigger.kind === 'app' ? 'app' : 'terminal'" :size="15"/></span>
        <span class="trigger-copy">
          <span class="trigger-sentence"><strong>{{ trigger.name }}</strong> is running</span>
          <span class="trigger-status"><span class="status-dot"/>{{ status(trigger) }}</span>
        </span>
        <AppTooltip text="Remove"><button class="icon-button trigger-remove" :aria-label="`Remove ${trigger.name}`" :disabled="busy" @click="remove(trigger.id)"><AppIcon name="close" :size="13"/></button></AppTooltip>
        <AppSwitch :model-value="trigger.enabled" :aria-label="`Stay awake while ${trigger.name} is running`" :disabled="busy" @update:model-value="update(trigger.id, { enabled: $event })"/>
      </li>
    </ul>
    <form v-if="adding" class="trigger-form" @submit.prevent="add" @keydown.esc.stop.prevent="close">
      <div class="segmented" role="radiogroup" aria-label="Trigger type">
        <button type="button" role="radio" :aria-checked="kind === 'app'" :class="{ selected: kind === 'app' }" @click="choose('app')"><AppIcon name="app" :size="14"/>App</button>
        <button type="button" role="radio" :aria-checked="kind === 'process'" :class="{ selected: kind === 'process' }" @click="choose('process')"><AppIcon name="terminal" :size="14"/>Command</button>
      </div>
      <template v-if="kind === 'app'">
        <AppSelect v-if="appOptions.length" v-model="appPath" class="app-picker" label="Running app" placeholder="Choose a running app" :options="appOptions"/>
        <p v-else class="card-empty">No other apps are running. Open the app first, or add its command.</p>
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
