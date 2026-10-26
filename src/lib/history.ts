import type { Day } from './model.ts'
import { shortDuration } from './session.ts'

export type Bar = { date: string; label: string; initial: string; duration: string; height: number; today: boolean }

function localDate(date: string): Date {
  const [year = 1970, month = 1, day = 1] = date.split('-').map(Number)
  return new Date(year, month - 1, day)
}

export function minutesText(minutes: number): string {
  return shortDuration(minutes * 60_000)
}

/** Whether there is anything to show for the last seven days. */
export function hasHistory(days: Day[]): boolean {
  return days.some(day => day.minutes > 0)
}

export function totalMinutes(days: Day[]): number {
  return days.reduce((sum, day) => sum + day.minutes, 0)
}

/** One bar per day, scaled to the busiest day; a day with any time keeps a visible sliver. */
export function bars(days: Day[]): Bar[] {
  const most = Math.max(1, ...days.map(day => day.minutes))
  return days.map((day, index) => {
    const date = localDate(day.date)
    return {
      date: day.date,
      label: date.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric' }),
      initial: date.toLocaleDateString(undefined, { weekday: 'narrow' }),
      duration: minutesText(day.minutes),
      height: day.minutes === 0 ? 0 : Math.max(0.08, day.minutes / most),
      today: index === days.length - 1,
    }
  })
}
