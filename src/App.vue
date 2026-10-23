<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue'
import { z } from 'zod'
import { TooltipProvider } from 'reka-ui'
import AppIcon from './components/AppIcon.vue'
import AppSwitch from './components/ui/AppSwitch.vue'
import AppTooltip from './components/ui/AppTooltip.vue'
import DurationPicker from './components/DurationPicker.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import TriggersCard from './components/TriggersCard.vue'
import OthersCard from './components/OthersCard.vue'
import { othersSummary } from './lib/others'
import StatusCandle from './components/StatusCandle.vue'
import { useWick } from './lib/wick'
import { useUpdater } from './lib/updater'
import { createDemoUpdateState } from './lib/demo'
import { activePreset, detail, fraction, shortDuration, title } from './lib/session'
import type { StartRequest } from './lib/model'
import { footerNote } from './lib/power'
import { isWindows } from './lib/platform'

const { state, pending, message, now, demo, desktop, startSession, stopSession, resumeTriggers, saveTriggers, runningApps, setDisplay, saveSettings, resetStore, hideWindow } = useWick()
const query = new URLSearchParams(location.search)
const updater = useUpdater({ demo, desktop, initial: demo ? createDemoUpdateState(query.get('state')) : undefined })
const placementSchema = z.object({ revision: z.number().int().nonnegative(), arrowX: z.number().finite().nullable() })
const placement = ref<z.infer<typeof placementSchema>>({ revision: 0, arrowX: null })
const viewportWidth = ref(window.innerWidth)
const media = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(media.matches)
const dark = computed(() => state.value.settings.theme === 'dark' || (state.value.settings.theme === 'system' && systemDark.value))
const page = ref<'overview' | 'settings'>(query.get('page') === 'settings' ? 'settings' : 'overview')
const startRequest = computed<StartRequest>(() => state.value.settings.startMinutes === null ? { mode: 'indefinite' } : { mode: 'for', minutes: state.value.settings.startMinutes })
const startHint = computed(() => state.value.settings.startMinutes === null ? 'Keep awake until you stop it' : `Keep awake for ${shortDuration(state.value.settings.startMinutes * 60_000)}`)
const busy = computed(() => pending.value !== null)
const on = computed(() => state.value.session.mode !== 'off')
const held = computed(() => !on.value && state.value.heldBy !== null)
const statusTitle = computed(() => title(state.value.session, now.value, state.value.heldBy))
const statusDetail = computed(() => state.value.pausedByLock && (on.value || held.value) ? 'Paused while the screen is locked' : state.value.session.mode === 'off' && !held.value && state.value.notice ? state.value.notice : !on.value && !held.value && !state.value.pausedBy && state.value.others.length ? othersSummary(state.value.others) ?? '' : detail(state.value.session, now.value, state.value.settings.keepDisplayOn, state.value.heldBy, state.value.pausedBy))
const footer = computed(() => footerNote(state.value.power, state.value.lidSleeps, on.value || held.value))
const ringFraction = computed(() => fraction(state.value.session, now.value))
const active = computed(() => activePreset(state.value.session))
const arrowStyle = computed(() => {
  const x = placement.value.arrowX
  if (!desktop || demo || isWindows || x === null || x < 26 || x > viewportWidth.value - 26) return undefined
  return { left: `${x - 6}px` }
})
const shell = ref<HTMLElement | null>(null)
const popover = ref<HTMLElement | null>(null)
const scroller = ref<HTMLElement | null>(null)
const fullHeight = 640
let fitHeight = 0
let fitFrame = 0
let fitObserver: ResizeObserver | undefined
let stopPlacement: (() => void) | undefined
let stopSettings: (() => void) | undefined
let destroyed = false

watchEffect(() => {
  document.documentElement.dataset.theme = dark.value ? 'dark' : 'light'
  document.documentElement.style.colorScheme = dark.value ? 'dark' : 'light'
  document.documentElement.dataset.platform = isWindows ? 'windows' : 'mac'
})

function measureFit(): void {
  fitFrame = 0
  if (!shell.value || !popover.value) return
  const style = getComputedStyle(shell.value)
  const body = scroller.value
  const content = body ? popover.value.offsetHeight - body.clientHeight + body.scrollHeight : popover.value.scrollHeight
  const height = page.value === 'settings' ? fullHeight : Math.min(fullHeight, Math.ceil(parseFloat(style.paddingTop) + parseFloat(style.paddingBottom) + content))
  if (height === fitHeight) return
  fitHeight = height
  if (desktop && !demo) window.__TAURI__?.core.invoke('fit_popover', { height }).catch(() => {})
}

function scheduleFit(): void {
  if (!fitFrame) fitFrame = requestAnimationFrame(measureFit)
}

function observeFit(): void {
  if (fitObserver) {
    fitObserver.disconnect()
    for (const element of [popover.value, scroller.value, ...(scroller.value?.children ?? [])]) if (element) fitObserver.observe(element)
  }
  scheduleFit()
}

async function quit(): Promise<void> {
  if (desktop && !demo) await window.__TAURI__?.core.invoke('quit').catch(() => {})
}

function onKeydown(event: KeyboardEvent): void {
  if (event.defaultPrevented) return
  if (event.key === 'Escape') {
    if (page.value === 'settings') page.value = 'overview'
    else void hideWindow()
  }
  if ((event.metaKey || event.ctrlKey) && event.key === ',') { event.preventDefault(); page.value = 'settings' }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'q') { event.preventDefault(); void quit() }
}

function acceptPlacement(payload: unknown): void {
  const result = placementSchema.safeParse(payload)
  if (!result.success) placement.value = { ...placement.value, arrowX: null }
  else if (result.data.revision >= placement.value.revision) placement.value = result.data
}

function syncSystemTheme(event: MediaQueryListEvent): void { systemDark.value = event.matches }
function syncViewportWidth(): void { viewportWidth.value = window.innerWidth }

watch([state, message, page], () => { void nextTick(observeFit) }, { deep: true })

onMounted(async () => {
  media.addEventListener('change', syncSystemTheme)
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('resize', syncViewportWidth)
  if (typeof ResizeObserver === 'function') {
    fitObserver = new ResizeObserver(scheduleFit)
  }
  observeFit()
  if (!desktop || demo) return
  const tauri = window.__TAURI__!
  tauri.event.listen('open_settings', () => { page.value = 'settings' }).then(unlisten => { if (destroyed) unlisten(); else stopSettings = unlisten }).catch(() => {})
  try {
    const unlisten = await tauri.event.listen('popover_placement_changed', event => acceptPlacement(event.payload))
    if (destroyed) unlisten()
    else stopPlacement = unlisten
    acceptPlacement(await tauri.core.invoke('get_popover_placement'))
  } catch { }
})

onUnmounted(() => {
  destroyed = true
  fitObserver?.disconnect()
  cancelAnimationFrame(fitFrame)
  stopPlacement?.()
  stopSettings?.()
  media.removeEventListener('change', syncSystemTheme)
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', syncViewportWidth)
})
</script>

<template>
  <TooltipProvider :delay-duration="500">
  <div :class="desktop && !demo ? 'desktop-stage' : 'preview-stage'">
    <div ref="shell" class="app-shell" :class="{ 'fit-content': page === 'overview' }">
      <div class="popover-arrow" :style="arrowStyle" aria-hidden="true"/>
      <div ref="popover" class="popover">
        <template v-if="page === 'overview'">
        <main ref="scroller" class="popover-body">
          <section class="hero" :class="{ active: on }">
            <StatusCandle :fraction="ringFraction" :on="on" :held="held"/>
            <div class="hero-copy" role="status" aria-live="polite">
              <h1 class="hero-title">{{ statusTitle }}</h1>
              <p class="hero-detail" :class="{ 'is-notice': !on && !held && state.notice }">{{ statusDetail }}<template v-if="!on && !held && state.pausedBy"> · <button class="text-button inline-action" :disabled="busy" @click="resumeTriggers">Resume</button></template></p>
            </div>
            <AppTooltip v-if="held" text="Pause this trigger until it ends"><button class="button hero-action" :disabled="busy" @click="stopSession">Stop</button></AppTooltip>
            <button v-else-if="on" class="button hero-action" :disabled="busy" @click="stopSession">Stop</button>
            <AppTooltip v-else :text="startHint"><button class="button button-primary hero-action" :disabled="busy" @click="startSession(startRequest)">Start</button></AppTooltip>
          </section>
          <div v-if="state.recovery" class="error-note" role="alert"><AppIcon name="alert" :size="16"/><div><p><strong>Settings could not be read.</strong> Wick is using defaults.</p><button class="text-button" @click="page = 'settings'">Review in Settings</button></div></div>
          <div v-if="state.error" class="error-note" role="alert"><AppIcon name="alert" :size="16"/><p><strong>Could not keep your Mac awake.</strong> {{ state.error }}</p></div>
          <section class="card" aria-labelledby="duration-label">
            <h2 id="duration-label" class="card-label">Keep awake for</h2>
            <DurationPicker :active="active" :busy="busy" :now="now" @start="startSession"/>
          </section>
          <OthersCard v-if="state.others.length" :others="state.others" :also="on || held"/>
          <TriggersCard :triggers="state.triggers" :running="state.running" :busy="busy" :save="saveTriggers" :load-apps="runningApps"/>
          <section class="card">
            <div class="setting-row card-row">
              <div>
                <label for="keep-display">Keep display on</label>
                <p>The screen won’t dim or lock.</p>
              </div>
              <AppSwitch id="keep-display" :model-value="state.settings.keepDisplayOn" :disabled="busy" @update:model-value="setDisplay"/>
            </div>
          </section>
        </main>
        <footer class="app-footer">
          <span class="footer-note"><template v-if="footer"><AppIcon :name="footer.icon" :size="15"/><span class="footer-text">{{ footer.text }}</span></template></span>
          <div class="footer-actions">
            <AppTooltip :text="updater.available.value ? 'Settings · update available' : 'Settings'"><button class="icon-button settings-button" :aria-label="updater.available.value ? 'Settings, update available' : 'Open settings'" @click="page = 'settings'"><AppIcon name="settings" :size="16"/><span v-if="updater.available.value" class="update-dot" aria-hidden="true"/></button></AppTooltip>
            <AppTooltip text="Quit Wick"><button class="icon-button" aria-label="Quit Wick" :title="isWindows ? 'Ctrl+Q' : '⌘Q'" @click="quit"><AppIcon name="power" :size="16"/></button></AppTooltip>
          </div>
        </footer>
        </template>
        <SettingsPanel v-else :update-state="updater.state.value" :update-busy="updater.busy.value" :demo="demo" :settings="state.settings" :power="state.power" :recovered="state.recovered" :recovery="state.recovery" :busy="busy" :save="saveSettings" :reset="resetStore" @back="page = 'overview'" @check-update="updater.check" @install-update="updater.install"/>
        <div v-if="demo" class="preview-ribbon"><span class="preview-dot"/>Interface preview<span>Sample data</span></div>
        <Transition name="toast"><div v-if="message" class="toast-message" :class="{ 'toast-error': message.kind === 'error' }" :role="message.kind === 'error' ? 'alert' : 'status'"><AppIcon :name="message.kind === 'error' ? 'alert' : 'check'" :size="15"/><span>{{ message.text }}</span><button class="icon-button" aria-label="Dismiss message" @click="message = null"><AppIcon name="close" :size="13"/></button></div></Transition>
      </div>
    </div>
  </div>
  </TooltipProvider>
</template>
