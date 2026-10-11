# Night fill comparison — 2026-10-11

Runtime WebGPU captures of the 850-building review fixture, high detail, simulation
paused, camera held in Explore after the skyline transition settled. Building and
neighborhood queues were empty before comparison. These are rendered screenshots,
not generated concepts.

Before/after pairs use a temporary developer button that changes only the
hemisphere intensity and ground color, within the same renderer instance and
fixed camera. The button labels are visible in the evidence. It was removed from
the final source. Animated presentation effects can still differ between frames;
these are matched compositions, not pixel-identical temporal captures.

- `before-miniature-19.jpg` / `after-miniature-19.jpg`: evening Miniature.
- `before-anime-19.jpg` / `after-anime-19.jpg`: evening Anime.
- `before-anime-12.jpg` / `after-anime-12.jpg`: noon Anime control.
- `final-anime-19.jpg`: separate final-build capture after removing the toggle;
  this is not part of the matched A/B pair.

## Accepted narrow change

Night hemisphere intensity rises from 0.65 to 1.05; daytime stays at 0.5.
Ground bounce transitions from neutral `#a0a09a` at night to the original
`#748998` in daylight. The existing smoothed daylight value drives both changes.
Sun/moon key, sky, exposure, local lights, window activity, water and AA retain
their existing behavior; no extra lights, passes, textures or geometry are added.

Primary visual review finds better separation of opaque facade planes and more
readable quiet streets in both styles, with warm windows still prominent. The
noon pair retains the existing daylight appearance. This is a modest legibility
improvement, not overall art acceptance: the scene still has large empty green
interiors, repeated architecture, weak street richness and an overly dark river
at night. Dense Tokyo fabric and the approved concept mood remain unmet.

Production build/typecheck passes (existing bundle-size warning). This change
has not been performance-benchmarked or reviewed on the WebGL fallback/mobile;
no frame-rate improvement or complete daily-cycle acceptance is claimed.
