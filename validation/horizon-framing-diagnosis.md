# Horizon and world-edge framing diagnosis — 2026-10-13

Current decision: do not add more distant mountains as an assumed remedy. Inspect
city-to-surroundings continuity and camera composition together. City-wide street
and frontage diagnosis remains required; this audit does not override it.

## Measured geometry

`scripts/audit-horizon-framing.mjs` imports production camera/lens/backdrop helpers.
The report `horizon-framing-audit.json` covers seeds87231/0/5 at tick10000 and their
retained mature station-exit checkpoints, each at35/50/85mm and1280×720. It derives
the settled Overview desired pose directly from developedExtent/frameOverview.
It does not run OrbitControls or a camera transition and is not browser readback.

- Overview looks downward23.73394degrees. Its top-center ray remains below the
  horizontal horizon by4.80930degrees at35mm,10.23821degrees at50mm and15.69823degrees
  at85mm. Thus all18poses omit the horizontal horizon; adding geometry at infinity
  cannot make it visible without a camera/framing change.
- The actual backdrop's outer boundary has7–24vertices in the35mm frustum across
  these worlds. At50/85mm no sampled boundary vertices enter. Vertex inclusion
  does not prove visible pixels: triangles can cross a frustum with no vertices
  inside, and buildings/terrain can occlude them.
- Production backdrop spans±1700m, begins relief outside450m and reaches full
  relief by1000m. Mountain rings are at2500/2750/3000m. The report samples a
  conservative upper envelope, not their true silhouette. Sky radius1800m does
  not by itself occlude mountains: sky depthWrite is disabled.

## Source observations and hypotheses

The finite city is surrounded by terrain rather than continuing urban fabric.
The latest user review rejects its visible green-plane/boundary appearance.
The source shows land at±420, an outer terrain square, and distant mountain rings.
A visible empty transition or terrain edge is plausible, but source alone cannot
assign the user's observed boundary to a specific mesh. Likewise the sky's
existence cannot establish a convincing horizon in steep views.

A delegated read-only audit independently identified finite backdrop termination
and low mountain profiles as hypotheses. Its rough camera-angle estimates were
replaced with the exact production calculations above. No production source was
changed. This evidence rejects an untested assumption that simply adding a
background skyline is sufficient; it does not reject horizon integration as work.

Next: inspect ordinary Overview and lower skyline views at matched checkpoints,
then reconcile with projected corridor/ground coverage and frontage continuity.
Choose one scene-scale correction from that evidence. Keep the review matrix,
normal growth, current saves, both styles and artistic acceptance requirements.

## Current browser observation

Ordinary seed0/tick58300, Anime,35mm,1280×720,High/WebGPU,DOFoff,noon preview,
rain0.82715786: `screenshots/citywide-review/seed0-tick58300-overview-anime-noon.png`.
The overview visibly places a rectangular developed city against broad empty
surrounding hills, with a long straight river continuation and dominant elevated
corridors. Background relief is visible, so absence of mountain geometry is not
the sole issue. The urban-to-landscape transition lacks convincing continuity.
This supports the user's broader criticism rather than dismissing it as merely
waterfront framing. This is one intermediate-maturity seed, not the full matrix.
Renderer details after capture show zero geometry queues,1024draws and13,347,916
triangles; one rolling window reports frame p95 46.8ms and longest51.8ms. These are
unisolated UI observations, not named-hardware performance acceptance.

The lower View skyline composition from the same paused world also exposes broad
empty hills behind the city and a large elevated deck in front. Saved as
`screenshots/citywide-review/seed0-tick58300-skyline-anime-noon.png` after switching
to Explore to hold the view. Its numerical pose was not exported; treat it as
qualitative context, not an exact future A/B baseline. This makes a mountains-only
fix less persuasive. Next measure city-wide corridor/frontage relationships and
select a coherent urban-edge/layout correction, retaining the full review matrix.
