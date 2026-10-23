import { z } from 'zod'

const millis = z.number().int().nonnegative()

export const sessionSchema = z.discriminatedUnion('mode', [
  z.object({ mode: z.literal('off') }),
  z.object({ mode: z.literal('for'), startedAt: millis, endsAt: millis }),
  z.object({ mode: z.literal('until'), startedAt: millis, endsAt: millis }),
  z.object({ mode: z.literal('indefinite'), startedAt: millis }),
])

export const settingsSchema = z.object({
  theme: z.enum(['system', 'light', 'dark']),
  keepDisplayOn: z.boolean(),
  startMinutes: z.number().int().min(1).max(7 * 24 * 60).nullable(),
  iconClick: z.enum(['open', 'toggle']),
  launchAtLogin: z.boolean(),
  shortcut: z.string().max(64).nullable(),
  batteryGuard: z.boolean(),
  batteryThreshold: z.number().int().min(5).max(80),
  stopInLowPowerMode: z.boolean(),
  notifyOnEnd: z.boolean(),
  autoCheckUpdates: z.boolean(),
})

export const powerSchema = z.object({
  onBattery: z.boolean(),
  percent: z.number().int().min(0).max(100).nullable(),
  lowPowerMode: z.boolean(),
})

export const triggerSchema = z.object({
  id: z.string().regex(/^[A-Za-z0-9-]{1,64}$/),
  kind: z.enum(['app', 'process']),
  name: z.string().min(1).max(64),
  path: z.string().max(1024).optional(),
  enabled: z.boolean(),
})

export const otherSchema = z.object({
  pid: z.number().int(),
  name: z.string().max(256),
  display: z.boolean(),
  reason: z.string().max(256).nullable(),
  parent: z.string().max(256).nullable(),
})

export const runningAppSchema = z.object({ name: z.string(), path: z.string() })

export const appStateSchema = z.object({
  revision: z.number().int().nonnegative(),
  session: sessionSchema,
  settings: settingsSchema,
  error: z.string().nullable(),
  notice: z.string().nullable(),
  power: powerSchema,
  lidSleeps: z.boolean().nullable(),
  triggers: z.array(triggerSchema),
  running: z.array(z.string()),
  heldBy: z.string().nullable(),
  pausedBy: z.string().nullable(),
  others: z.array(otherSchema),
  recovery: z.string().nullable(),
  recovered: z.array(z.string()),
})

export const startRequestSchema = z.discriminatedUnion('mode', [
  z.object({ mode: z.literal('for'), minutes: z.number().int().min(1).max(7 * 24 * 60) }),
  z.object({ mode: z.literal('until'), hour: z.number().int().min(0).max(23), minute: z.number().int().min(0).max(59) }),
  z.object({ mode: z.literal('indefinite') }),
])

export type Session = z.infer<typeof sessionSchema>
export type Settings = z.infer<typeof settingsSchema>
export type Power = z.infer<typeof powerSchema>
export type Trigger = z.infer<typeof triggerSchema>
export type RunningApp = z.infer<typeof runningAppSchema>
export type Other = z.infer<typeof otherSchema>
export type AppState = z.infer<typeof appStateSchema>
export type StartRequest = z.infer<typeof startRequestSchema>

export const defaultSettings: Settings = { theme: 'system', keepDisplayOn: false, startMinutes: null, iconClick: 'open', launchAtLogin: false, shortcut: null, batteryGuard: true, batteryThreshold: 20, stopInLowPowerMode: false, notifyOnEnd: true, autoCheckUpdates: true }

export const initialState = (): AppState => ({ revision: 0, session: { mode: 'off' }, settings: { ...defaultSettings }, error: null, notice: null, power: { onBattery: false, percent: null, lowPowerMode: false }, lidSleeps: null, triggers: [], running: [], heldBy: null, pausedBy: null, others: [], recovery: null, recovered: [] })
