import type { StartRequest } from './model.ts'

export type KeyAction = { kind: 'start'; request: StartRequest } | { kind: 'toggle' } | { kind: 'extend'; minutes: number }

/** Single keys in the popover. Nothing while typing or with Command, Control or Option held. */
export function keyAction(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey'>, typing: boolean): KeyAction | null {
  if (typing || event.metaKey || event.ctrlKey || event.altKey) return null
  switch (event.key) {
    case '1': return { kind: 'start', request: { mode: 'for', minutes: 15 } }
    case '2': return { kind: 'start', request: { mode: 'for', minutes: 60 } }
    case '3': return { kind: 'start', request: { mode: 'for', minutes: 120 } }
    case '0': return { kind: 'start', request: { mode: 'indefinite' } }
    case 's': case 'S': return { kind: 'toggle' }
    case '+': case '=': return { kind: 'extend', minutes: 15 }
    default: return null
  }
}

export function isTyping(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
}
