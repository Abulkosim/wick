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

/** Typing into a field, or into a picker whose typeahead jumps to an option. */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName) || target.closest('[role="combobox"], [role="listbox"]') !== null
}

/** Arrow keys move through a radio group: -1 back, 1 forward. */
export function radioStep(key: string): -1 | 0 | 1 {
  if (key === 'ArrowRight' || key === 'ArrowDown') return 1
  if (key === 'ArrowLeft' || key === 'ArrowUp') return -1
  return 0
}

/** Moves focus and selection to the next radio in the group the event came from. */
export function moveRadio(event: KeyboardEvent): void {
  const step = radioStep(event.key)
  if (!step || !(event.currentTarget instanceof HTMLElement)) return
  const radios = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]:not(:disabled)')]
  if (!radios.length) return
  const index = radios.findIndex(radio => radio === document.activeElement)
  const next = radios[(Math.max(index, 0) + step + radios.length) % radios.length]
  if (!next) return
  event.preventDefault()
  next.focus()
  next.click()
  // Choosing can move focus to the field it reveals; arrows keep it in the group.
  requestAnimationFrame(() => next.focus())
}
