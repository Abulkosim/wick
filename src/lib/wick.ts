import { onMounted, onUnmounted, ref } from 'vue'
import { createDemoState } from './demo.ts'
import { appStateSchema, initialState, type AppState, type StartRequest } from './model.ts'
import { start } from './session.ts'

type Command = 'start_session' | 'stop_session' | 'set_display'

export function useWick() {
  const query = new URLSearchParams(window.location.search)
  const demo = query.get('demo') === '1'
  const desktop = typeof window.__TAURI__?.core.invoke === 'function'
  const state = ref<AppState>(demo ? createDemoState(query.get('state')) : initialState())
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

  const startSession = (request: StartRequest) => perform('start_session', { request }, () => { state.value.session = start(request, Date.now()) })
  const stopSession = () => perform('stop_session', undefined, () => { state.value.session = { mode: 'off' } })
  const setDisplay = (display: boolean) => perform('set_display', { display }, () => { state.value.display = display })

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

  return { state, loading, pending, message, now, demo, desktop, startSession, stopSession, setDisplay, hideWindow, notify }
}
