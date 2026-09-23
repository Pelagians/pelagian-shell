const specialWindowRoles = new Set([
  'overlay',
  'hud',
  'splash',
  'tooltip',
  'menu',
  'notification',
  'utility'
])

/** True when the containing Shell session owns top-level window chrome. */
export function pelagianShellUsesServerDecorations(env = process.env) {
  return env.PELAGIAN_SHELL_WINDOW_CHROME === 'server'
}

/** Stable policy value for consumers that also need to adapt their renderer. */
export function pelagianShellWindowChromePolicy(env = process.env) {
  return pelagianShellUsesServerDecorations(env) ? 'server' : 'application'
}

/**
 * Adapt ordinary Electron BrowserWindow options to the active Pelagian Shell
 * policy. The caller must label intentional frameless surfaces explicitly.
 * Parent or modal windows are treated as ordinary decorated dialogs.
 */
export function applyPelagianShellWindowChrome(options, { role, env = process.env } = {}) {
  if (!pelagianShellUsesServerDecorations(env)) return options

  const windowRole = role ?? (options.parent || options.modal ? 'dialog' : 'ordinary')
  if (specialWindowRoles.has(windowRole)) return options
  if (windowRole !== 'ordinary' && windowRole !== 'dialog') {
    throw new TypeError(`unknown Pelagian Shell window role: ${windowRole}`)
  }

  const adapted = { ...options, frame: true }
  delete adapted.titleBarStyle
  delete adapted.titleBarOverlay
  return adapted
}
