# Status

Replace, don't append. Keep under 2 KB. History lives in dated `validation/<topic>.md` files and `validation/archive/`.

## In progress

- Architecture refactor ([plan](../specs/architecture-refactor-plan.md)): the large
  mechanical moves land first, before parallel work resumes. Rebase open branches
  after it merges.

## Product milestones (from specs/08-delivery.md)

1. Camera, shaders and rendering: **active, not accepted.** Reference-led optics,
   focus, materials, lighting, atmosphere and water, stable in motion, in both styles.
2. Convincing reference neighborhood: pending.
3. Consistent city character: pending.
4. Cinematic presentation across the city: pending.
5. Complete living experience and release: pending.

## Open gates

- Master-plan acceptance: **not passing.** The legacy planners stay, gated behind
  the `legacy` cargo feature, until it passes.
- Visual acceptance needs captures compared with the locked references; tests alone
  never accept a visual change.
- Ramps, merges and close-up transit motion are unfinished.

## Last checkpoint

PR #20 (camera rendering checkpoint) merged. Next visual work: coherent
street/building/landscape composition at ordinary viewing scale. See
[archive/2026-10/HANDOFF.md](archive/2026-10/HANDOFF.md) for the history.
