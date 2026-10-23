import { onMounted, onUnmounted, ref } from 'vue'
import { createDemoState, demoApps } from './demo.ts'
import { z } from 'zod'
import { appStateSchema, defaultSettings, initialState, runningAppSchema, type AppState, type RunningApp, type Settings, type StartRequest, type Trigger } from './model.ts'
import { start } from './session.ts'
import { stopReason } from './power.ts'

type Command = 'start_session' | 'stop_session' | 'set_display' | 'save_settings' | 'reset_store' | 'save_triggers' | 'resume_triggers'

export function useWick() {
  const query = new URLSearchParams(window.location.search)
  const demo = query.get('demo') === '1'
  const desktop = typeof window.__TAURI__?.core.invoke === 'function'
  const state = ref<AppState>(demo ? createDemoState(query.get('state')) : initialState())
  const theme = query.get('theme')
  if (demo && (theme === 'light' || theme === 'dark')) state.value.settings.theme = theme
  const loading = ref(desktop && !demo)
  const pending = ref<Command | null>(null)
  const message = ref<{ kind: 'error' | 'success'; text: string } | null>(null)
  const now = ref(Date.now())
  let stopListening: (() => void) | undefined
  let destroyed = false
  let toastTimer: ReturnType<typeof setTimeout> | undefined
  const clock = window.setInterval(() => { now.value = Date.now() }, 1000)

  function notify(text: string, kind: 'error' | 'success' = 'success'): void {
    clearTimeout(toastTimer)
    message.value = { kind, text }
    toastTimer = setTimeout(() => { message.value = null }, kind === 'error' ? 9000 : 3500)
  }

  function accept(payload: unknown): void {
    const result = appStateSchema.safeParse(payload)
    if (!result.success) throw new Error('Wick received an unreadable state. Reopen the app to reconnect.')
    if (result.data.revision >= state.value.revision) state.value = result.data
    now.value = Date.now()
  }

  async function perform(command: Command, args: Record<string, unknown> | undefined, preview: () => void): Promise<boolean> {
    if (pending.value) return false
    pending.value = command
    try {
      if (demo || !desktop) {
        preview()
        state.value = { ...state.value, revision: state.value.revision + 1, error: null }
      } else {
        accept(await window.__TAURI__!.core.invoke(command, args))
      }
      return true
    } catch (error: unknown) {
      notify(error instanceof Error ? error.message : typeof error === 'string' ? error : 'That did not work. Try again.', 'error')
      return false
    } finally {
      pending.value = null
    }
  }

  const startSession = (request: StartRequest) => perform('start_session', { request }, () => {
    const reason = stopReason(state.value.power, state.value.settings)
    if (reason) throw new Error(`Not started: ${reason.replace('Stopped: ', '')}. Plug in, or change it in Settings → Battery.`)
    state.value.session = start(request, Date.now())
    state.value.notice = null
  })
  const stopSession = () => perform('stop_session', undefined, () => {
    if (state.value.session.mode === 'off' && state.value.heldBy) {
      state.value.pausedBy = state.value.heldBy.replace(/ is running$/, ' quits').replace(/^on power$/, 'you unplug').replace(/^an external display is connected$/, 'the display is disconnected')
      state.value.heldBy = null
    }
    state.value.session = { mode: 'off' }
    state.value.notice = null
  })
  const resumeTriggers = () => perform('resume_triggers', undefined, () => {
    if (state.value.pausedBy) state.value.heldBy = state.value.pausedBy.replace(/ quits$/, ' is running').replace(/^you unplug$/, 'on power').replace(/^the display is disconnected$/, 'an external display is connected')
    state.value.pausedBy = null
  })
  const saveTriggers = (triggers: Trigger[]) => perform('save_triggers', { triggers }, () => {
    state.value.triggers = triggers
    state.value.running = state.value.running.filter(id => triggers.some(trigger => trigger.id === id && trigger.enabled))
    const holding = triggers.find(trigger => trigger.enabled && state.value.running.includes(trigger.id))
    if (state.value.heldBy && !holding) state.value.heldBy = null
  })

  async function runningApps(): Promise<RunningApp[]> {
    if (demo || !desktop) return demoApps
    try {
      return z.array(runningAppSchema).parse(await window.__TAURI__!.core.invoke('running_apps'))
    } catch {
      notify('Could not list running apps. Type a process name instead.', 'error')
      return []
    }
  }
  const setDisplay = (display: boolean) => perform('set_display', { display }, () => { state.value.settings = { ...state.value.settings, keepDisplayOn: display } })
  const saveSettings = (settings: Settings) => perform('save_settings', { settings }, () => { state.value.settings = settings; state.value.recovered = [] })

  async function resetStore(): Promise<boolean> {
    const reset = await perform('reset_store', undefined, () => {
      state.value.settings = { ...defaultSettings }
      state.value.recovery = null
      state.value.recovered = []
    })
    if (reset) notify('Settings reset. The old file was kept as a backup.')
    return reset
  }

  async function hideWindow(): Promise<void> {
    if (!desktop || demo) return
    await window.__TAURI__!.core.invoke('hide_window').catch(() => {})
  }

  onMounted(async () => {
    if (!desktop || demo) return
    const tauri = window.__TAURI__!
    try {
      const unlisten = await tauri.event.listen('state_changed', event => {
        try { accept(event.payload) } catch (error) { notify((error as Error).message, 'error') }
      })
      if (destroyed) unlisten()
      else stopListening = unlisten
      accept(await tauri.core.invoke('get_state'))
    } catch {
      notify('Could not connect to Wick. Quit and reopen the app.', 'error')
    } finally {
      loading.value = false
    }
  })

  onUnmounted(() => {
    destroyed = true
    stopListening?.()
    clearInterval(clock)
    clearTimeout(toastTimer)
  })

  return { state, loading, pending, message, now, demo, desktop, startSession, stopSession, resumeTriggers, saveTriggers, runningApps, setDisplay, saveSettings, resetStore, hideWindow, notify }
}
