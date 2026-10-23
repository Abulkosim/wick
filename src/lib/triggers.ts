export const processSuggestions = ['claude', 'codex', 'ffmpeg', 'xcodebuild']

export function validProcessName(value: string): boolean {
  const name = value.trim()
  return name.length > 0 && name.length <= 64 && !name.includes('/') && !/[\u0000-\u001f\u007f]/.test(name)
}

export function triggerId(): string {
  return `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
}
