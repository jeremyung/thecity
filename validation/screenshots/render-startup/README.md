# Rendering startup investigation — 2026-10-11

`founder-software-webgl.png` is an actual local browser screenshot of the founding
city, using Chromium headless with ANGLE SwiftShader and `?webgl=1`, 1000×700.
It was captured before the startup handshake correction. It shows that this
fallback could render the shared scene on this host. The probe observed only
five completed frames at its first inspection; no controlled frame-rate benchmark
was taken. This is neither hardware WebGPU validation nor mature-city acceptance.

The preceding WebGPU attempt loaded the 871-building world but returned no frame
metrics and timed out taking a screenshot. This investigation does not establish
its cause. Do not use the fallback image to claim that failure is fixed.

Source inspection did establish a separate readiness bug: `start()` resolved
after scheduling the loop, before the first render submission. The startup
watchdog therefore stopped early, and later synchronous frame exceptions escaped
its error handler. Startup now awaits the first successful JS frame submission;
this is not proof of GPU completion/presentation. A first-frame exception rejects
startup; a later exception stops the loop and reports through the existing UI
recovery controls. Disposal settles a pending startup and prevents further frame
callbacks. The simulation remains independent and can still be saved.

Validation: root independently passes 11 lifecycle/startup/recovery tests and the
production build/typecheck (existing large-bundle warning). A fresh software
WebGL browser session reached `first-frame-submitted`; injecting a synchronous
exception into the active pipeline produced `frame-error`, the message
“Graphics stopped: injected frame failure”, visible Retry controls, and a
successful `__city.save()` response. The injected failure existed only in that
isolated browser session. Startup failure and disposal paths have unit coverage;
GPU device loss/asynchronous driver errors are not covered by this exception path.
