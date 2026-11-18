<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import type { AgentConnection } from '../../lib/model'
import { connectionAction, connectionStatus, trustFile } from '../../lib/agents'
import { device } from '../../lib/platform'
import AppIcon from '../AppIcon.vue'
import AppSwitch from '../ui/AppSwitch.vue'

export type AgentBridge = {
  list: () => Promise<AgentConnection[]>
  connect: (agent: AgentConnection['agent']) => Promise<AgentConnection[]>
  disconnect: (agent: AgentConnection['agent']) => Promise<AgentConnection[]>
}

const props = defineProps<{ enabled: boolean; settingsBusy: boolean; bridge: AgentBridge }>()
const emit = defineEmits<{ enabled: [value: boolean] }>()
const connections = ref<AgentConnection[] | null>(null)
const loadError = ref<string | null>(null)
const working = ref<AgentConnection['agent'] | null>(null)
const confirming = ref<AgentConnection['agent'] | null>(null)
const errors = ref<Partial<Record<AgentConnection['agent'], string>>>({})
const labels = { connect: 'Connect', disconnect: 'Disconnect', reconnect: 'Reconnect' } as const
const configVariables = { claude: 'CLAUDE_CONFIG_DIR', codex: 'CODEX_HOME' } as const

async function load(): Promise<void> {
  loadError.value = null
  try {
    connections.value = await props.bridge.list()
  } catch (error) {
    loadError.value = (error as Error).message
  }
}

function focusAction(agent: AgentConnection['agent'], selector = '.connection-action'): void {
  void nextTick(() => document.querySelector<HTMLButtonElement>(`[data-agent="${agent}"] ${selector}`)?.focus())
}

async function run(connection: AgentConnection, action: 'connect' | 'disconnect'): Promise<void> {
  if (working.value) return
  working.value = connection.agent
  errors.value = { ...errors.value, [connection.agent]: undefined }
  try {
    connections.value = await (action === 'connect' ? props.bridge.connect : props.bridge.disconnect)(connection.agent)
    confirming.value = null
  } catch (error) {
    errors.value = { ...errors.value, [connection.agent]: (error as Error).message }
    confirming.value = null
  } finally {
    working.value = null
    focusAction(connection.agent)
  }
}

function activate(connection: AgentConnection): void {
  const action = connectionAction(connection)
  if (action === 'connect') {
    confirming.value = connection.agent
    focusAction(connection.agent, '.confirm-connect')
  } else {
    void run(connection, action === 'reconnect' ? 'connect' : 'disconnect')
  }
}

function cancel(agent: AgentConnection['agent']): void {
  confirming.value = null
  focusAction(agent)
}

onMounted(load)
</script>

<template>
  <section class="settings-section agent-settings" aria-labelledby="agents-title">
    <h2 id="agents-title">Agents</h2>
    <div class="setting-row"><div><span id="agents-enabled-label" class="setting-label">Keep awake while agents work</span><p>Lets your {{ device }} sleep when they finish or wait for you.</p></div><AppSwitch :model-value="enabled" aria-labelledby="agents-enabled-label" :disabled="settingsBusy" @update:model-value="emit('enabled', $event)"/></div>
    <div class="connection-card">
      <p v-if="loadError" class="connection-load-error" role="alert"><span>{{ loadError }}</span><button class="text-button inline-action" @click="load">Try again</button></p>
      <p v-else-if="!connections" class="card-empty" role="status">Checking agents…</p>
      <ul v-else class="connection-list" aria-label="Agent connections">
        <li v-for="connection in connections" :key="connection.agent" class="connection-row" :data-agent="connection.agent" :class="[`is-${connection.status}`, { confirming: confirming === connection.agent, 'has-note': connection.status === 'connected' && connection.error }]">
          <div class="connection-main">
            <span class="connection-mark"><AppIcon name="spark" :size="15"/></span>
            <div class="connection-copy">
              <span :id="`connection-${connection.agent}`" class="setting-label">{{ connection.name }}</span>
              <span class="connection-status" :role="connection.status === 'error' ? 'alert' : undefined"><span v-if="connection.status !== 'error'" class="status-dot"/>{{ connectionStatus(connection) }}</span>
            </div>
            <button v-if="confirming !== connection.agent" class="button button-small connection-action" :class="{ 'button-primary': connectionAction(connection) === 'reconnect' }" :disabled="connection.status === 'missing' || working !== null" :aria-busy="working === connection.agent" :aria-describedby="`connection-${connection.agent}`" @click="activate(connection)">{{ working === connection.agent ? 'Working…' : labels[connectionAction(connection)] }}</button>
          </div>
          <div v-if="confirming === connection.agent" class="connection-confirm" role="group" :aria-label="`Connect ${connection.name}`" @keydown.esc.stop.prevent="cancel(connection.agent)">
            <p v-if="trustFile(connection) === null">Adds hooks to <code>{{ connection.file }}</code>. The file is backed up first.</p>
            <p v-else-if="trustFile(connection) === connection.file">Adds hooks to <code>{{ connection.file }}</code> and marks them trusted there. The file is backed up first.</p>
            <p v-else>Adds hooks to <code>{{ connection.file }}</code> and marks them trusted in <code>{{ trustFile(connection) }}</code>. Both files are backed up first.</p>
            <p class="connection-note">Wick reads <code>{{ configVariables[connection.agent] }}</code> from its own environment, not from your shell profile.</p>
            <div class="form-actions">
              <button class="button button-small" @click="cancel(connection.agent)">Cancel</button>
              <button class="button button-primary button-small confirm-connect" :disabled="working !== null" @click="run(connection, 'connect')">{{ working === connection.agent ? 'Connecting…' : 'Connect' }}</button>
            </div>
          </div>
          <p v-if="errors[connection.agent] && errors[connection.agent] !== connectionStatus(connection)" class="field-error connection-error" role="alert">{{ errors[connection.agent] }}</p>
        </li>
      </ul>
    </div>
    <p class="connection-hint">Other agents and scripts: <code>wick run -- &lt;command&gt;</code></p>
  </section>
</template>

<style scoped>
.connection-card { margin-top: 10px; padding: 4px 14px; background: var(--card); border-radius: 9px; }
.connection-card>.card-empty, .connection-load-error { padding: 12px 0; }
.connection-load-error { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; font-size: 13px; color: var(--danger); line-height: 1.5; }
.connection-load-error .text-button { color: var(--ink); font-weight: 500; }
.connection-list { list-style: none; margin: 0; padding: 0; }
.connection-row { padding: 10px 0; }
.connection-row + .connection-row { border-top: 1px solid var(--line); }
.connection-main { display: flex; align-items: center; gap: 10px; }
.connection-mark { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 7px; background: var(--surface); color: var(--secondary); flex-shrink: 0; }
.is-connected .connection-mark { background: var(--accent-soft); color: var(--accent-ink); }
.connection-copy { display: flex; flex-direction: column; flex: 1; min-width: 0; }
.connection-copy .setting-label { font-weight: 500; line-height: 1.35; }
.connection-status { font-size: 12px; line-height: 1.45; color: var(--tertiary); overflow-wrap: anywhere; }
.connection-status .status-dot { display: inline-block; margin: 0 5px 2px 0; vertical-align: middle; background: var(--track); }
.is-connected .connection-status { color: var(--accent-ink); }
.is-connected .connection-status .status-dot { background: var(--accent); }
.is-stale .connection-status, .is-connected.has-note .connection-status { color: var(--warning); }
.is-stale .connection-status .status-dot, .is-connected.has-note .connection-status .status-dot { background: var(--warning); }
.is-error .connection-status { color: var(--danger); }
.is-missing .setting-label { color: var(--tertiary); }
.connection-action { flex-shrink: 0; min-width: 92px; }
.connection-action:disabled { opacity: .5; cursor: default; }
.connection-confirm { margin: 10px 0 2px 38px; font-size: 13px; line-height: 1.5; color: var(--secondary); }
.connection-confirm .form-actions { margin-top: 9px; }
.connection-note { margin-top: 4px; font-size: 12px; color: var(--tertiary); }
.connection-error { margin: 6px 0 0 38px; }
.connection-hint { margin-top: 10px; padding: 0 2px; font-size: 13px; line-height: 1.5; color: var(--tertiary); }
code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; color: var(--secondary); overflow-wrap: anywhere; }
</style>
