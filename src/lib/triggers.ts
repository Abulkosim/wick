export const processSuggestions = ['claude', 'codex', 'ffmpeg', 'xcodebuild']

export function validProcessName(value: string): boolean {
  const name = value.trim()
  return name.length > 0 && name.length <= 64 && !name.includes('/') && !/[\u0000-\u001f\u007f]/.test(name)
}

export function triggerId(): string {
  return `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}

export const dayLetters = ['M', 'T', 'W', 'T', 'F', 'S', 'S']
const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

/** Mirrors Rust's `Schedule::days_label`. */
export function daysLabel(days: number[]): string {
  const sorted = [...new Set(days)].sort((a, b) => a - b)
  const key = sorted.join(',')
  if (key === '0,1,2,3,4,5,6') return 'Every day'
  if (key === '0,1,2,3,4') return 'Weekdays'
  if (key === '5,6') return 'Weekends'
  return sorted.map(day => dayNames[day]).join(', ')
}

export function validSchedule(days: number[], start: string, end: string): string | null {
  if (!days.length) return 'Choose at least one day.'
  const time = /^([01]?\d|2[0-3]):[0-5]\d$/
  if (!time.test(start) || !time.test(end)) return 'Use times like 09:00.'
  if (start === end) return 'Start and end can’t be the same time.'
  return null
}
