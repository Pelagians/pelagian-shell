import assert from 'node:assert/strict'
import test from 'node:test'

import {
  applyPelagianShellWindowChrome,
  pelagianShellUsesServerDecorations,
  pelagianShellWindowChromePolicy
} from '../integrations/electron/window-chrome.mjs'

test('Shell policy is a single stable environment contract', () => {
  assert.equal(pelagianShellUsesServerDecorations({ PELAGIAN_SHELL_WINDOW_CHROME: 'server' }), true)
  assert.equal(pelagianShellUsesServerDecorations({ PELAGIAN_SHELL_WINDOW_CHROME: 'client' }), false)
  assert.equal(pelagianShellUsesServerDecorations({}), false)
  assert.equal(pelagianShellWindowChromePolicy({ PELAGIAN_SHELL_WINDOW_CHROME: 'server' }), 'server')
  assert.equal(pelagianShellWindowChromePolicy({}), 'application')
})

test('ordinary windows use the compositor frame and drop Electron overlay chrome', () => {
  const input = {
    width: 800,
    titleBarStyle: 'hidden',
    titleBarOverlay: { color: '#000', symbolColor: '#fff' }
  }
  const result = applyPelagianShellWindowChrome(input, {
    env: { PELAGIAN_SHELL_WINDOW_CHROME: 'server' }
  })
  assert.deepEqual(result, { width: 800, frame: true })
  assert.equal(input.titleBarStyle, 'hidden', 'the adapter does not mutate caller options')
})

test('parented and modal dialogs retain decorated behavior', () => {
  assert.deepEqual(
    applyPelagianShellWindowChrome(
      { parent: {}, modal: true, frame: false },
      { env: { PELAGIAN_SHELL_WINDOW_CHROME: 'server' } }
    ),
    { parent: {}, modal: true, frame: true }
  )
  assert.deepEqual(
    applyPelagianShellWindowChrome(
      { modal: true, titleBarStyle: 'hidden' },
      { env: { PELAGIAN_SHELL_WINDOW_CHROME: 'server' } }
    ),
    { modal: true, frame: true }
  )
})

test('explicit overlays and utility surfaces keep their intentional frameless options', () => {
  for (const role of ['overlay', 'hud', 'splash', 'tooltip', 'menu', 'notification', 'utility']) {
    const options = { frame: false, transparent: true, titleBarOverlay: false }
    assert.equal(
      applyPelagianShellWindowChrome(options, {
        role,
        env: { PELAGIAN_SHELL_WINDOW_CHROME: 'server' }
      }),
      options
    )
  }
})

test('unknown roles fail closed', () => {
  assert.throws(
    () => applyPelagianShellWindowChrome({}, {
      role: 'unknown',
      env: { PELAGIAN_SHELL_WINDOW_CHROME: 'server' }
    }),
    /unknown Pelagian Shell window role/
  )
})
