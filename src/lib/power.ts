import type { Power, Settings } from './model.ts'

export function stopReason(power: Power, settings: Pick<Settings, 'batteryGuard' | 'batteryThreshold' | 'stopInLowPowerMode'>): string | null {
  if (!power.onBattery) return null
  if (settings.batteryGuard && power.percent !== null && power.percent < settings.batteryThreshold) return `Stopped: battery below ${settings.batteryThreshold}%`
  if (settings.stopInLowPowerMode && power.lowPowerMode) return 'Stopped: Low Power Mode is on'
  return null
}

export function powerNote(power: Power): string | null {
  if (power.lowPowerMode) return 'Low Power Mode is on'
  if (power.onBattery && power.percent !== null) return `On battery · ${power.percent}%`
  return null
}

export const hasBattery = (power: Power): boolean => power.percent !== null
