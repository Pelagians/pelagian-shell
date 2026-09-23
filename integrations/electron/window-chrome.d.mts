export type PelagianShellWindowRole =
  | 'ordinary'
  | 'dialog'
  | 'overlay'
  | 'hud'
  | 'splash'
  | 'tooltip'
  | 'menu'
  | 'notification'
  | 'utility'

export type PelagianShellWindowChromePolicy = 'server' | 'application'

export function pelagianShellUsesServerDecorations(env?: NodeJS.ProcessEnv): boolean
export function pelagianShellWindowChromePolicy(env?: NodeJS.ProcessEnv): PelagianShellWindowChromePolicy
export function applyPelagianShellWindowChrome<T extends object>(
  options: T,
  context?: { role?: PelagianShellWindowRole; env?: NodeJS.ProcessEnv }
): T
