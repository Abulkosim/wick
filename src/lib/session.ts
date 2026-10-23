import type { Session, StartRequest } from './model.ts'

export type Preset = { id: string; label: string; request: StartRequest }

export const presets: Preset[] = [
  { id: '15m', label: '15 min', request: { mode: 'for', minutes: 15 } },
  { id: '1h', label: '1 hour', request: { mode: 'for', minutes: 60 } },
  { id: '2h', label: '2 hours', request: { mode: 'for', minutes: 120 } },
]

export function shortDuration(milliseconds: number): string {
  const minutes = Math.ceil(Math.max(0, milliseconds) / 60_000)
  if (minutes < 60) return `${minutes}m`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ${String(minutes % 60).padStart(2, '0')}m`
  return `${Math.floor(hours / 24)}d ${hours % 24}h`
}

export function clockTime(time: number, now: number): string {
  const date = new Date(time)
  const today = new Date(now)
  const sameDay = date.toDateString() === today.toDateString()
  const options: Intl.DateTimeFormatOptions = sameDay ? { hour: 'numeric', minute: '2-digit' } : { weekday: 'short', hour: 'numeric', minute: '2-digit' }
  return date.toLocaleString(undefined, options)
}

export function remaining(session: Session, now: number): number | null {
  return 'endsAt' in session ? Math.max(0, session.endsAt - now) : null
}

export function fraction(session: Session, now: number): number {
  if (session.mode === 'off') return 0
  if (session.mode === 'indefinite') return 1
  return Math.min(1, Math.max(0, (session.endsAt - now) / Math.max(1, session.endsAt - session.startedAt)))
}

export function title(session: Session, now: number, heldBy: string | null = null): string {
  switch (session.mode) {
    case 'off': return heldBy ? 'Awake' : 'Off'
    case 'indefinite': return 'Awake'
    case 'until': return `Awake until ${clockTime(session.endsAt, now)}`
    case 'for': return `Awake · ${shortDuration(session.endsAt - now)} left`
  }
}

export function detail(session: Session, now: number, display: boolean, heldBy: string | null = null, pausedBy: string | null = null): string {
  const screen = display ? ' · Display on' : ''
  switch (session.mode) {
    case 'off': return heldBy ? `While ${heldBy}${screen}` : pausedBy ? `Paused until ${pausedBy}` : 'Your Mac sleeps as usual.'
    case 'indefinite': return `Until you stop it${screen}`
    case 'until': return `${shortDuration(session.endsAt - now)} left${screen}`
    case 'for': return `Until ${clockTime(session.endsAt, now)}${screen}`
  }
}

export function activePreset(session: Session): string | null {
  if (session.mode === 'indefinite') return 'indefinite'
  if (session.mode === 'until') return 'until'
  if (session.mode === 'for') {
    const minutes = Math.round((session.endsAt - session.startedAt) / 60_000)
    return presets.find(preset => preset.request.mode === 'for' && preset.request.minutes === minutes)?.id ?? 'custom'
  }
  return null
}

export function parseClock(value: string): { hour: number; minute: number } | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim())
  if (!match) return null
  const hour = Number(match[1])
  const minute = Number(match[2])
  return hour < 24 && minute < 60 ? { hour, minute } : null
}

export function customMinutes(hours: number, minutes: number): number | null {
  if (!Number.isInteger(hours) || !Number.isInteger(minutes) || hours < 0 || minutes < 0) return null
  const total = hours * 60 + minutes
  return total >= 1 && total <= 7 * 24 * 60 ? total : null
}

export function start(request: StartRequest, now: number): Session {
  if (request.mode === 'indefinite') return { mode: 'indefinite', startedAt: now }
  if (request.mode === 'for') return { mode: 'for', startedAt: now, endsAt: now + request.minutes * 60_000 }
  const end = new Date(now)
  end.setHours(request.hour, request.minute, 0, 0)
  if (end.getTime() <= now) end.setDate(end.getDate() + 1)
  return { mode: 'until', startedAt: now, endsAt: end.getTime() }
}
