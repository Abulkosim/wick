const modifierOrder = ['Control', 'Alt', 'Shift', 'Super'] as const
const symbols: Record<(typeof modifierOrder)[number], string> = { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' }
const keyNames: Record<string, string> = {
  Space: 'Space', Enter: '↩', Tab: '⇥', Backspace: '⌫', Delete: '⌦', Escape: '⎋',
  ArrowUp: '↑', ArrowDown: '↓', ArrowLeft: '←', ArrowRight: '→',
  Minus: '-', Equal: '=', BracketLeft: '[', BracketRight: ']', Backslash: '\\', Semicolon: ';', Quote: "'", Comma: ',', Period: '.', Slash: '/', Backquote: '`',
}
const modifierCodes = new Set(['ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight', 'CapsLock', 'Fn'])

export type ShortcutEvent = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>

export type Recorded = { kind: 'shortcut'; value: string } | { kind: 'pending' } | { kind: 'error'; message: string }

export function recordShortcut(event: ShortcutEvent): Recorded {
  if (modifierCodes.has(event.code) || !event.code) return { kind: 'pending' }
  const keyName = /^(Key[A-Z]|Digit\d|F\d{1,2})$/.test(event.code) || event.code in keyNames ? event.code : null
  if (!keyName) return { kind: 'error', message: 'That key can’t be used in a shortcut.' }
  if (!event.ctrlKey && !event.altKey && !event.metaKey) return { kind: 'error', message: 'Include Command, Control or Option.' }
  const modifiers = modifierOrder.filter(name => ({ Control: event.ctrlKey, Alt: event.altKey, Shift: event.shiftKey, Super: event.metaKey })[name])
  return { kind: 'shortcut', value: [...modifiers, keyName].join('+') }
}

export function formatShortcut(value: string): string {
  const parts = value.split('+')
  const key = parts.at(-1) ?? ''
  const modifiers = modifierOrder.filter(name => parts.includes(name)).map(name => symbols[name]).join('')
  const label = key.startsWith('Key') ? key.slice(3) : key.startsWith('Digit') ? key.slice(5) : keyNames[key] ?? key
  return `${modifiers}${label}`
}
