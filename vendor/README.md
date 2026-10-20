# Slint Windows software rendering workaround

`i-slint-backend-winit` is copied from the published Slint 1.18.1 crate,
including its original copyright and license files.

Local change: on Windows, `renderer/sw.rs` uses `NewBuffer` for each requested
frame. This repaints pixels invalidated by DPI transitions and window restore,
even when softbuffer reports a reused buffer. It does not introduce a timer or
render loop; the tray has no settings window until requested. Painting an open
settings window costs a full frame instead of partial damage.

Upstream tracking: https://github.com/slint-ui/slint/issues/13537 and
https://github.com/slint-ui/slint/pull/13568 . Remove this patch after upgrading
to a released version containing the upstream fix and verifying mixed-DPI moves.

## winit 0.30.13: mixed-DPI drag growth

`winit` is copied from its published 0.30.13 crate, with its original licenses.
The Windows WM_DPICHANGED path follows upstream #4341: Windows 11 (build >=
22000) uses the OS suggested rectangle directly, bypassing legacy monitor
nudging that triggers repeated DPI changes and runaway growth. Windows 10 keeps
the legacy adjustment, but selects its target monitor from the suggested
rectangle rather than the window's previous monitor (#4704). Version detection
shares the existing cached RtlGetVersion result and initializes the structure
size explicitly.

Sources: https://github.com/rust-windowing/winit/pull/4341 and
https://github.com/rust-windowing/winit/pull/4704 . These are a local backport,
not an upgrade to unreleased winit APIs. Remove after a compatible release fixes
mixed-DPI dragging and both directions are verified on real monitors.
