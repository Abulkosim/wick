<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue'
import { z } from 'zod'
import { TooltipProvider } from 'reka-ui'
import AppIcon from './components/AppIcon.vue'
import AppSwitch from './components/ui/AppSwitch.vue'
import AppTooltip from './components/ui/AppTooltip.vue'
import DurationPicker from './components/DurationPicker.vue'
import StatusRing from './components/StatusRing.vue'
import { useWick } from './lib/wick'
import { activePreset, detail, fraction, title } from './lib/session'

const { state, pending, message, now, demo, desktop, startSession, stopSession, setDisplay, hideWindow } = useWick()
const query = new URLSearchParams(location.search)
const placementSchema = z.object({ revision: z.number().int().nonnegative(), arrowX: z.number().finite().nullable() })
const placement = ref<z.infer<typeof placementSchema>>({ revision: 0, arrowX: null })
const viewportWidth = ref(window.innerWidth)
const media = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(media.matches)
const theme = query.get('theme')
const dark = computed(() => theme === 'dark' || (theme !== 'light' && systemDark.value))
const busy = computed(() => pending.value !== null)
const on = computed(() => state.value.session.mode !== 'off')
const statusTitle = computed(() => title(state.value.session, now.value))
const statusDetail = computed(() => detail(state.value.session, now.value, state.value.display))
const ringFraction = computed(() => fraction(state.value.session, now.value))
const active = computed(() => activePreset(state.value.session))
const arrowStyle = computed(() => {
  const x = placement.value.arrowX
  if (!desktop || demo || x === null || x < 26 || x > viewportWidth.value - 26) return undefined
  return { left: `${x - 6}px` }
})
const shell = ref<HTMLElement | null>(null)
const popover = ref<HTMLElement | null>(null)
const fullHeight = 560
let fitHeight = 0
let fitFrame = 0
let fitObserver: ResizeObserver | undefined
let stopPlacement: (() => void) | undefined
let destroyed = false

watchEffect(() => {
  document.documentElement.dataset.theme = dark.value ? 'dark' : 'light'
  document.documentElement.style.colorScheme = dark.value ? 'dark' : 'light'
})

function measureFit(): void {
  fitFrame = 0
  if (!shell.value || !popover.value) return
  const style = getComputedStyle(shell.value)
  const height = Math.min(fullHeight, Math.ceil(parseFloat(style.paddingTop) + parseFloat(style.paddingBottom) + popover.value.scrollHeight))
  if (height === fitHeight) return
  fitHeight = height
  if (desktop && !demo) window.__TAURI__?.core.invoke('fit_popover', { height }).catch(() => {})
}

function scheduleFit(): void {
  if (!fitFrame) fitFrame = requestAnimationFrame(measureFit)
}

async function quit(): Promise<void> {
  if (desktop && !demo) await window.__TAURI__?.core.invoke('quit').catch(() => {})
}

function onKeydown(event: KeyboardEvent): void {
  if (event.defaultPrevented) return
  if (event.key === 'Escape') void hideWindow()
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'q') { event.preventDefault(); void quit() }
}

function acceptPlacement(payload: unknown): void {
  const result = placementSchema.safeParse(payload)
  if (!result.success) placement.value = { ...placement.value, arrowX: null }
  else if (result.data.revision >= placement.value.revision) placement.value = result.data
}

function syncSystemTheme(event: MediaQueryListEvent): void { systemDark.value = event.matches }
function syncViewportWidth(): void { viewportWidth.value = window.innerWidth }

watch(() => [state.value.error, message.value], () => { void nextTick(scheduleFit) })

onMounted(async () => {
  media.addEventListener('change', syncSystemTheme)
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('resize', syncViewportWidth)
  if (typeof ResizeObserver === 'function') {
    fitObserver = new ResizeObserver(scheduleFit)
    if (popover.value) fitObserver.observe(popover.value)
    for (const child of popover.value?.children ?? []) fitObserver.observe(child)
  }
  scheduleFit()
  if (!desktop || demo) return
  const tauri = window.__TAURI__!
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
  media.removeEventListener('change', syncSystemTheme)
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', syncViewportWidth)
})
</script>

<template>
  <TooltipProvider :delay-duration="500">
  <div :class="desktop && !demo ? 'desktop-stage' : 'preview-stage'">
    <div ref="shell" class="app-shell fit-content">
      <div class="popover-arrow" :style="arrowStyle" aria-hidden="true"/>
      <div ref="popover" class="popover">
        <main class="popover-body">
          <section class="hero" :class="{ active: on }">
            <StatusRing :fraction="ringFraction" :on="on"/>
            <div class="hero-copy" role="status" aria-live="polite">
              <h1 class="hero-title">{{ statusTitle }}</h1>
              <p class="hero-detail">{{ statusDetail }}</p>
            </div>
            <button v-if="on" class="button hero-action" :disabled="busy" @click="stopSession">Stop</button>
            <AppTooltip v-else text="Keep awake until you stop it"><button class="button button-primary hero-action" :disabled="busy" @click="startSession({ mode: 'indefinite' })">Start</button></AppTooltip>
          </section>
          <div v-if="state.error" class="error-note" role="alert"><AppIcon name="alert" :size="16"/><p><strong>Could not keep your Mac awake.</strong> {{ state.error }}</p></div>
          <section class="card" aria-labelledby="duration-label">
            <h2 id="duration-label" class="card-label">Keep awake for</h2>
            <DurationPicker :active="active" :busy="busy" :now="now" @start="startSession"/>
          </section>
          <section class="card">
            <div class="setting-row card-row">
              <div>
                <label for="keep-display">Keep display on</label>
                <p>The screen won’t dim or lock.</p>
              </div>
              <AppSwitch id="keep-display" :model-value="state.display" :disabled="busy" @update:model-value="setDisplay"/>
            </div>
          </section>
        </main>
        <footer class="app-footer">
          <span class="footer-note"/>
          <div class="footer-actions">
            <AppTooltip text="Quit Wick"><button class="icon-button" aria-label="Quit Wick" @click="quit"><AppIcon name="power" :size="16"/></button></AppTooltip>
          </div>
        </footer>
        <div v-if="demo" class="preview-ribbon"><span class="preview-dot"/>Interface preview<span>Sample data</span></div>
        <Transition name="toast"><div v-if="message" class="toast-message" :class="{ 'toast-error': message.kind === 'error' }" :role="message.kind === 'error' ? 'alert' : 'status'"><AppIcon :name="message.kind === 'error' ? 'alert' : 'check'" :size="15"/><span>{{ message.text }}</span><button class="icon-button" aria-label="Dismiss message" @click="message = null"><AppIcon name="close" :size="13"/></button></div></Transition>
      </div>
    </div>
  </div>
  </TooltipProvider>
</template>
