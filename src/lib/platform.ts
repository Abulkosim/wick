export type Platform = 'mac' | 'windows' | 'linux'

const search = typeof location === 'undefined' ? '' : location.search
const query = new URLSearchParams(search)
const userAgent = typeof navigator === 'undefined' ? '' : navigator.userAgent

const platforms: readonly Platform[] = ['mac', 'windows', 'linux']
const injected = (globalThis as { __WICK_PLATFORM__?: unknown }).__WICK_PLATFORM__

/**
 * The preview picks a platform with `?platform=windows|linux`; the desktop app is told by Rust
 * (`__WICK_PLATFORM__`), with the user agent as a fallback.
 */
export const platform: Platform = query.get('demo') === '1' || !('__TAURI__' in globalThis)
  ? (['windows', 'linux'] as const).find(name => query.get('platform') === name) ?? 'mac'
  : platforms.find(name => name === injected)
    ?? (userAgent.includes('Windows') ? 'windows' : userAgent.includes('Linux') || userAgent.includes('X11') ? 'linux' : 'mac')

export const isWindows = platform === 'windows'
export const isLinux = platform === 'linux'
export const isMac = platform === 'mac'

/** What the copy calls this computer, matching the Rust side. */
export const device = isMac ? 'Mac' : isWindows ? 'PC' : 'computer'
