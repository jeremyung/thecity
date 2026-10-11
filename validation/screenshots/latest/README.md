# Runtime visual checkpoint — 2026-10-10

Captured through the in-app browser on local build `a9dd390`, WebGPU, high detail,
with the development **Review mature city** fixture (850 buildings). Simulation
was paused after loading; both render queues were empty before capture. The
skyline camera and city are shared between shots; lighting previews differ.
These are runtime screenshots, not generated concept art or the 853-building
final state from the separate worker continuation.

- `miniature-afternoon.jpg`: Miniature, preview hour 16.
- `anime-evening.jpg`: Anime, preview hour 19.

Neither meets the visual release gate. The current city has excessive empty
green interiors, repetitive facades, a broad pale afternoon river reflection,
and weakly illuminated evening streets compared with the approved concepts.

Follow-up must distinguish missing simulated building fabric from rendering:
the terrain uses continuous meadow, while open building parcels already receive
local paving in `web/src/render/kit.ts`. Random decorative buildings or trees
must not disguise an underdeveloped simulation. Candidate rendering work is
more varied envelope-safe facade families, water reflection contrast, and
balanced night street/facade illumination. Each requires matched runtime visual
review; these observations alone do not establish a successful fix.
