import type { Other } from './model.ts'
import { device } from './platform.ts'

export function otherDetail(other: Other): string {
  const base = other.parent ? `Started by ${other.parent}` : other.reason ?? (other.display ? 'Keeps the display on' : `Keeps the ${device} awake`)
  return other.display && (other.parent || other.reason) ? `${base} · Display on` : base
}

/** What the status line says when Wick is off but other apps hold the computer awake. */
export function othersSummary(others: Other[]): string | null {
  const first = others[0]
  if (!first) return null
  if (others.length === 1) return `${first.parent ?? first.name} is keeping your ${device} awake.`
  return `${others.length} apps are keeping your ${device} awake.`
}
