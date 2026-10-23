import { z } from 'zod'

const millis = z.number().int().nonnegative()

export const sessionSchema = z.discriminatedUnion('mode', [
  z.object({ mode: z.literal('off') }),
  z.object({ mode: z.literal('for'), startedAt: millis, endsAt: millis }),
  z.object({ mode: z.literal('until'), startedAt: millis, endsAt: millis }),
  z.object({ mode: z.literal('indefinite'), startedAt: millis }),
])

export const appStateSchema = z.object({
  revision: z.number().int().nonnegative(),
  session: sessionSchema,
  display: z.boolean(),
  error: z.string().nullable(),
})

export const startRequestSchema = z.discriminatedUnion('mode', [
  z.object({ mode: z.literal('for'), minutes: z.number().int().min(1).max(7 * 24 * 60) }),
  z.object({ mode: z.literal('until'), hour: z.number().int().min(0).max(23), minute: z.number().int().min(0).max(59) }),
  z.object({ mode: z.literal('indefinite') }),
])

export type Session = z.infer<typeof sessionSchema>
export type AppState = z.infer<typeof appStateSchema>
export type StartRequest = z.infer<typeof startRequestSchema>

export const initialState = (): AppState => ({ revision: 0, session: { mode: 'off' }, display: false, error: null })
