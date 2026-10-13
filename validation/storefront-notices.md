# Shared storefront notices — 2026-10-11

The texture recheck found little benefit from additional generic wall grain.
The next bounded change addresses empty storefront glazing with authored paper
notices. One 512×512 canvas atlas contains four cafe/shop designs with Japanese
lettering; no image generation call or external asset is used. Explicit opening
hours were avoided so decoration does not invent operating schedules.

Each eligible display zone receives one plane inside its entrance-side bay, clear
of the entry and mullions. The first trial used the opposite bay and was obscured
by the vending machine; the retained placement moves it beside the entrance.
Two triangles per notice; at most two notices on the current centered shopfront.
The atlas and material variants are shared, with no per-building texture creation.
One RGBA8 mip-chain estimate is 1,398,100 bytes (1.33 MiB); this is not measured GPU
residency. A shared material can still add a batch to each affected chunk.
Paper stays non-emissive under both style shaders.

Verification: 11 low-rise kit checks and one material ownership/disposal check
pass. Production build passes with the existing chunk warning. The test's first
no-emission assertion was corrected to accept the material API's null node value;
no runtime emission behavior was changed to satisfy it.

Browser captures use the static architecture review, Inspect shop preset, 1280×720,
open stage, wall-texture candidate disabled, no DOF, both daylight styles plus
Miniature night. This specimen has no city seed/tick. Compare against
`validation/screenshots/texture-recheck/*-baseline.png`. New captures are in
`validation/screenshots/storefront-notices/`. The notice makes a modest readable
addition beside the entry and darkens naturally at night. It does not fix the
large-scale architectural or lighting gap.

Retained in shared shop geometry, pending full living-street motion, distance and
resource review. Do not expand the atlas library from this specimen result alone.
Reject further variants that require per-building textures or exaggerated poster
scale to show an improvement. The next acceptance check is a normal street view,
not another isolated material swatch.
