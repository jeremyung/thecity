# Shared shop-sign atlas — 2026-10-12

Previous goal turn progressed: PR10 review found a failing atlas-boundary test,
verified its build, and identified integration conflicts. This bounded follow-up
ports only its shop-sign batching idea into the current working tree, fixing
bounds. PR10 itself remains unchanged and unmerged.

## Deficiency and intervention

Each finished shop/cafe/anchor previously allocated a512×160 texture and its
own transparent material. Distinct material identities prevent signs merging
in neighborhood chunks. Signs have fully opaque paper, so a shared opaque
MeshBasicNodeMaterial preserves their intent while permitting batching.

ArtMaterials now owns one512×528 atlas: three160pixel sign cells with8pixel
padding above/below each. Height derives from label count×stride, fixing PR10's
third-cell clipping. Kit sign geometry maps its V coordinates into its cell;
source-group disposal releases geometry but leaves shared resources to ArtMaterials.
The separate storefront-poster atlas and its WebGPU binding correction are intact.
No camera, tree-clearance or shadow-cadence changes were ported. No paid assets.

## Verification and limits

Root reviewed Luna's tests, removed unnecessary casts and added explicit shared
material disposal assertions. 36checks pass across shop-sign-atlas, storefront-atlas,
building-chunks, lowrise-kit and tower-kit. They verify all padded cells stay in
bounds, style-invariant sharing, full cell UVs survive merging, three signs in
one chunk produce one sign mesh, and source/chunk disposal preserves resources
until ArtMaterials disposal. This is unit batching evidence, not a browser
frame-time measurement. A nullability error in the test texture spy was caught by TypeScript and fixed
with an explicit missing-atlas guard. Production build passes with existing
bundle warning after that correction.

Normal startup through the production scheduler, paused seed87231tick570,
10buildings, liveclock06:34. Current-format export retained at
.tools/fidelity-review/shop-sign-city.json. Explore after Watch, no manual orbit,
DOFoff,1280×720, noon lighting preview. Held-camera captures:
[Anime](screenshots/shop-sign-atlas/anime.png) and
[Miniature](screenshots/shop-sign-atlas/miniature.png).
Root inspected the café sign intact in both, waited for the style control to
confirm completion, and saw no captured errors/warnings. No original-material
matched baseline, numeric camera pose, other two labels in browser, continuous
motion, fallback or mature-city GPU cost has been verified here.

Retain provisionally as a measured batching improvement, not full visual or
performance acceptance. If mipmapped labels bleed or sign geometry/lifecycle
regresses under ordinary growth, correct the atlas or revert this narrow port.
Do not claim the PR's draw/frame figures for this newer working tree.
