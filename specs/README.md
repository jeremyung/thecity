# Tokyo Living Simulation Specification

A browser based, autonomous, watchable Tokyo inspired city grows from sparse settlement into a bounded metropolis. Rust compiled to WebAssembly owns the world; Three.js renders it with selectable artistic styles. Last Train, First Light ships first, followed by Little Tokyo. The viewer explores and watches rather than managing a game.

## Reading order

| Specification | Purpose |
| --- | --- |
| [Experience and scope](01-experience.md) | Product intent, viewer controls, states, and first release |
| [World growth and infrastructure](02-world-growth.md) | Finite geography, expansion, road building, redevelopment, and limits |
| [Simulation engine](03-simulation.md) | Entities, clocks, systems, behavior, and deterministic scheduling |
| [Browser architecture](04-architecture.md) | Rust WASM boundary, worker protocol, persistence, and extensibility |
| [Rendering and artistic styles](05-rendering.md) | Shared 3D assets, shader styles, anti-aliasing, water, and lighting |
| [Camera and scene direction](06-camera.md) | Gradual reveal, scene selection, manual exploration, and growth framing |
| [Sound and music](07-audio.md) | Spatial city sounds, lo-fi score, and scheduling |
| [Delivery and validation](08-delivery.md) | Milestones, budgets, tests, profiling, and acceptance criteria |

## Decision status

Implementation and verification status live in [the specification gap audit](../validation/spec-audit.md); the latest city capture and density evidence are in [the integration review](../validation/multiblock-integration.md#visual-review). Specifications describe intended behavior, not proof of implementation. The world-growth specification also records [procedural-generation research and proposed next work](02-world-growth.md#procedural-generation-research-and-proposed-next-work).

Confirmed direction: living observation experience; independent random cities with coherent internal histories; browser execution; shared simulation with selectable renderers; Last Train, First Light implemented first; Little Tokyo preferred visually; dynamic water and full-scene anti-aliasing; complete daily cycle; city sounds and indie lo-fi mood.

The geography dimensions, density, population, time rates, and resource budgets below are proposed defaults. Tune them against measured hardware and watchability. They are finite constraints, not promises of benchmark performance.

External morphology research: [PLATEAU findings and bounded pilot](../validation/plateau-research.md). Root-reviewed direction is procedural calibration first, with offline source processing and a shared cross-style benchmark. Three bounded Tokyo sources now have hash-pinned provenance, reconciled feature counts, extracted roof-edge metrics and diagrams in the [source pilot](../validation/plateau-pilot/README.md). District calibration, synthetic comparisons and visual acceptance remain pending; these samples are not production density targets.

## Governing decisions

1. A fixed landscape contains a changing city. No infinite terrain generation.
2. Road access and occupied destinations precede development; adjacency alone is insufficient.
3. Growth opportunities are evaluated locally with global capacity checks.
4. City maturity shifts activity toward reuse and renewal rather than unbounded expansion.
5. Camera changes never affect economic outcomes or random generation.
6. The simulation is authoritative and independent of rendering and audio.
7. Under load, reduce optional effect resolution, distant detail, and fast-forward throughput while preserving the focal scene, required water, and anti-aliasing. Visual fidelity remains a release requirement; performance pressure does not justify replacing the approved art direction with a generic city.
8. Background execution pauses; saves preserve continuity.

## Concept references

- [Last Train, First Light](../concepts/04-last-train-first-light.png)
- [Little Tokyo](../concepts/01-little-tokyo.png)
- [Painted Hours](../concepts/02-painted-hours.png)
- [Ink After Rain](../concepts/03-ink-after-rain.png)
- [Metropolitan waterways and block-pattern reference](../concepts/photo-references/metropolitan-waterways-block-patterns.png)
- [Image generation prompts](../concepts/prompts.txt)

Concept boards illustrate atmosphere and development stages. They are not executable rendering designs or evidence that the city develops within one day.
