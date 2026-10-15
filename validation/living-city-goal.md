Build the complete browser-based Tokyo living city defined by `README.md`, `specs/README.md`, specifications 01–08 and `.impeccable.md`, achieving the visual direction of the locked concepts `04-last-train-first-light.png` and `01-little-tokyo.png`.

Create a beautiful, believable, relaxing city that grows organically and remains rewarding to watch and explore. Infrastructure, buildings, construction, renewal and resident journeys must have coherent causes. Camera, lighting, landscape, water, spatial sound and music must support that experience.

Preserve the established architecture: Rust/WASM simulation in a deterministic fixed-step Web Worker; TypeScript/React/Vite; Three.js WebGPU/TSL; Web Audio; and IndexedDB. Keep systems modular, work bounded and current-format saves deterministic. Obsolete-save compatibility is unnecessary.

**Immediate milestone:** bring the existing 800 m city to convincing visual and behavioral quality before expanding maps or importing external city datasets. Both rendering styles must depict the same world, geometry, camera and time, and remain distinct with depth of field disabled.

Address the largest visual gaps in this order:

1. **City-wide urban fabric:** reduce excessive openness, regularity and road dominance. Establish narrow neighborhood streets, tight varied frontage, differentiated blocks and coherent curving road, rail and river corridors. Preserve legal access, clearance and continuous junctions.
2. **Composition and environmental depth:** resolve abrupt city edges and empty surroundings; improve camera composition, directional lighting and atmospheric separation.
3. **Architecture and materials:** establish recognizable Japanese building families, warm neutral palettes, varied silhouettes, believable facades, balconies, exterior stairs and roof equipment.
4. **Street identity and vegetation:** make utility infrastructure, signage, everyday frontage objects, trees and planting visibly contribute at ordinary viewing distances.
5. **Water and landscape:** improve water depth, glints, reflections, banks and crossings, with stable appearance in motion.
6. **Living activity and audio:** develop purposeful journeys, destination use, daily rhythms, construction, renewal and coherent sound. Increased activity does not replace improvements to the city’s visual structure.

Maintain **one primary workstream**, selected from the latest user review and current validation evidence. Each intervention must have a concrete deficiency, a bounded reusable change, matched before/after evidence and a stop condition.

Evaluate ordinary and weak views across three seeds, developing and mature states, and interior streets, frontages, stations, waterfronts and overviews. Hold simulation state, numerical camera pose, lens, lighting, weather, resolution and quality constant for comparisons. Separate geometry improvements from camera and lighting changes.

Avoid repeated tiny embellishments, arbitrary density increases, cosmetic crowds, broad texture batches, speculative rewrites and fog or blur that hides weak geometry. After two unsuccessful attempts, reassess the approach.

Validate through the production scheduler, including growth, loading, construction, renewal, deterministic replay, legal journeys, both styles, day/night transitions, camera motion, audio and resource stability. Measure performance on named hardware and browsers. Keep documentation and outstanding limitations accurate.

Completion requires the complete living-city experience, specification 08 acceptance, and user acceptance against the locked concepts. The 800 m milestone does not remove the broader map-scale requirements.
