/// <reference types="vite/client" />

interface Window {
  __TAURI__?: {
    core: { invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown> }
    event: { listen: (event: string, handler: (event: { payload: unknown }) => void) => Promise<() => void> }
  }
}
