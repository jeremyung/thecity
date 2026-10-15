# Shallow manual orbit — 2026-10-12

Previous turn progressed core facade geometry. This pass investigates camera
composition rather than adding more facade details. At the current50mm lens
(24mm sensor), verticalFOV is~27degrees. Old maximum polar angle1.32rad requires
at least14.37degrees downward elevation. With a ground target, the top ray remains
below horizontal: it cannot include the distant sky. An actual upward orbit drag
hit this limit, showing city roofs and a green backdrop but no sky; further drag
had no effect. This baseline was visually inspected in the tool, not saved as a
registered screenshot.

Changed only OrbitControls maximum polar angle1.32→1.44rad (~7.5degrees above
horizontal). Existing12mminimumheight and manual building-obstacle recovery
remain in place. The held diagnostic preset, Watchdirection/selection and all
lens values stay unchanged. This expands manual exploration, not automatic
cinematic framing or a safety guarantee for all worlds.

Normal import of full1839city,seed87231,tick75925,clear,paused09:55,Anime,
HighWebGPU,50mmDOFoff,1280×720,noonpreview. All geometryqueues0 before existing
Hold waterfront view; then drag(900,440)→(900,320) reached new shallow range.
[Capture](screenshots/shallow-orbit/anime-noon.png) shows sky and distant terrain,
with more readable elevations; nearby buildings also occlude part of the river.
Retain the added user control, but do not promote this as a composed Watch shot.
Camera coordinates were not exported, so this is an interaction check, not exact
registered pose evidence. The original held waterfront baseline remains available.

45existing camera tests and productionbuild pass (existing bundle warning).
These tests regress current framing/clearance logic; they are not a new exhaustive
shallow-angle collision test. Other lenses, styles, narrowviewport and fallback
still need interaction checks. Broader repeated massing/backdrop quality is more
obvious at this angle and remains unfinished. No commit/push. Goalactive.
Tab22leftpausedAnime/noon/fullcity at shallowmanualpose.
