# Pelagian Shell window chrome contract v0

Pelagian Shell owns top-level window chrome for its Wayland session. The canonical environment variable is:

```text
PELAGIAN_SHELL_WINDOW_CHROME=server
```

When it is `server`, ordinary top-level applications yield window chrome to the compositor. Labwc supplies the visible titlebar for solo, tiled, and floating ordinary windows. The titlebar shows the title and close button only. Shell owns maximization and automatic layout. The session has one workspace.

Dialogs and transient windows stay decorated and floating. A consumer may explicitly preserve frameless behavior for an overlay, HUD, splash screen, tooltip, menu, notification, or frameless utility. Shell does not infer those roles from application names or titles.

The normal Shell window has one compositor-owned titlebar. An application should not add duplicate minimize, maximize, or close controls, and should not reserve renderer space for Window Controls Overlay controls. Binary-only applications that cannot disable client chrome may document a compatibility exception; Shell does not claim to inspect arbitrary pixels and certify that exception.

## Electron adapter

`integrations/electron/window-chrome.mjs` is the Shell-owned reference adapter. It has no Electron runtime dependency. Use `pelagianShellWindowChromePolicy()` when a consumer renderer also needs to suppress its own top-level chrome. Call `applyPelagianShellWindowChrome()` only when constructing an ordinary application window, and label intentional special-purpose surfaces explicitly:

```js
new BrowserWindow(applyPelagianShellWindowChrome(options))

new BrowserWindow(applyPelagianShellWindowChrome(options, { role: 'dialog' }))

new BrowserWindow(applyPelagianShellWindowChrome(options, { role: 'overlay' }))
```

Under the Shell policy, ordinary windows and dialogs use Electron's default frame behavior. The adapter sets `frame: true` and removes `titleBarStyle` and `titleBarOverlay`. Special roles keep their supplied options. Applications remain responsible for hiding any renderer-owned window controls and removing Window Controls Overlay spacing when the policy is active.

Consumers should use the canonical Shell variable as their integration input. An upstream application may provide a generic setting of its own; the consumer adapter can translate the Shell contract to that setting when it launches the application.

## Runtime verification

`pelagian-shellctl status` reports `window_chrome_policy`. The reusable consumer session verifier checks the live Labwc decoration and health state. Electron source checks are an early gate for known source consumers; runtime checks verify the real window remains a normal Shell-managed window. Consumers should also remove renderer-owned top-level title strips and reserved Window Controls Overlay space while this policy is active. These checks do not claim to prove arbitrary third-party pixels.
