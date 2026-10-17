# Architecture refactor plan

Responds to the 2026-10-16 architecture review. Baseline: `origin/main` @ `236211a`.

## Review claims checked against main

| Claim | On main | Note |
|---|---|---|
| `lib.rs` 22,944 lines, tests ~10k | 22,944; `mod tests` starts at line 12,542 | Confirmed: ~10.4k test lines |
| 16 files `impl Engine` | 15 besides `lib.rs` | Close enough |
| Flat `engine/src` | 91 entries, 91 `mod` lines in `lib.rs` | Confirmed |
| `step_one()` hard-codes cadence | `tick % 10/40/200` at `lib.rs:2526+` | Confirmed |
| `new_legacy` slated for removal | `new()` is master plan; `new_legacy` is used by 71 sites in `lib.rs` and ~20 other files, mostly tests | Removing it means re-baselining or deleting those tests |
| 14 experiment/diagnostic modules | 17 file names match `*_diagnostic`, `*_experiment`, `*_trace` | Confirmed |
| `route_where` rebuilds adjacency | `Vec::with_capacity(roads.len()*2)` at `lib.rs:~9146` | Confirmed |
| Hand-written `types.ts` | 72 lines, string `kind`/`stage` | Confirmed; small, so cheap to generate |
| `city.ts` imports camera/audio | 6 imports, including `CameraDirector` | Confirmed |
| `city.ts` 92 KB | 1,229 lines, 18 lines over 300 chars, longest 1,424 | Size comes from line length, so a formatter fixes most of it |
| Movement stride duplicated | `MOVEMENT_STRIDE = 6`, `TRAIN_DOOR_STRIDE = 3` in `web/src/movement.ts` | Confirmed |
| `validation/` noise | 976 tracked files, 348 MB | Confirmed. Fixtures already come from release assets (`setup-test-fixtures.py`, `hydrate-review-evidence.py`), so #25 can reuse that path |
| No AGENTS.md/CLAUDE.md, no STATUS.md | None | Confirmed |
| Prettier/ESLint/rustfmt in CI | None installed; rustfmt state not measured yet | — |

## Ground rules for every step

- **One concern per PR**: refactor, behavior change, or visual change.
- **A refactor PR must pass these unchanged**: `npm run test:engine -- --release`,
  the master-plan replay (`scripts/audit/verify-masterplan-replay.mjs`), the save
  round-trip, `npm test` and `npm run build`. A failure means the refactor
  changed behavior.
- **Save format**: old saves may break (saves are not yet a compatibility
  promise). Same-version save → load → save must stay byte-stable. Where a
  refactor would change the serde shape, use `#[serde(flatten)]`,
  `#[serde(transparent)]` or `rename_all` to keep the shape, or bump the
  save version on purpose in its own PR.
- **Behavior changes** (#12, #13, #14, and #15 if it changes any outcome) re-baseline the
  replay fixtures and record before/after in `validation/<topic>.md`.
- Before each large mechanical move, merge or rebase any open branch that
  touches `lib.rs`, `city.ts`, `kit.ts` or `director.ts`.

## Step 0: Agent orientation (no code changes, about one day)

Cheap and unblocks everything else, so do it first.

- 0a. Add `AGENTS.md` (review block verbatim, with module map paths fixed to current
  reality) and `CLAUDE.md` = `@AGENTS.md`. Add `.ignore` skipping
  `validation/`, `art-workshop/`, `concepts/`, `public/`, `dist/`.
- 0b. Add `validation/STATUS.md` (under 2 KB: current priorities and open gates),
  drawn from HANDOFF.md and fidelity-review-current.md. Move both files to
  `validation/archive/2026-10/`.
- 0c. Strip dated sections out of `specs/*.md`, such as the 04-architecture
  2026-10-12 and 2026-10-11 checkpoints, into `validation/<topic>.md`.
  Cut README down to orientation. Add `ARCHITECTURE.md` (#27). Wait to write
  `CONTRIBUTING.md` until the layer and system patterns exist, so it describes
  real patterns.

## Phase 1: Clear the ground (no behavior change)

| PR | Items | Approach | Risk / gate |
|---|---|---|---|
| 1.1 | #4 (Rust) | `cargo fmt` once and add `cargo fmt --check` to CI. Add the commit to `.git-blame-ignore-revs` | Pure formatting |
| 1.2 | #4 (web) | Add Prettier (`printWidth: 100`), format all of `web/src` and `tests/` in one commit, add `prettier --check` to CI, add the commit to blame-ignore | Merge in-flight web branches first |
| 1.3 | #3 | Move `lib.rs mod tests` into `src/tests/<domain>.rs` as child modules (`#[cfg(test)] mod tests;`). They keep private access, so no API widening. Use `engine/tests/` only for tests that already use the public API | Test count must match before and after |
| 1.4 | #2 | Move the 17 diagnostic/experiment modules to `src/experiments/`, gated by `#[cfg(any(test, feature = "experiments"))]`. Move the ~30 `#[cfg(test)]` Engine fields into an `ExperimentState` side struct, owned by the harness and passed into the hooks that need it | Hooks that read those fields need a small injection seam |
| 1.5 | #1a | Put `new_legacy` and the frontier/infill/access-discovery/spine/coordinated-block planners behind `feature = "legacy"`. Enable it in `test:engine` only | Default build must not compile legacy code |
| 1.6 | #1b | **Decision gate**: once master-plan acceptance passes, delete the legacy feature. Port each of the ~90 `new_legacy` call sites to `new()` or delete it. Record in `validation/legacy-removal.md` which tests were dropped and why | Behavior is unchanged for the app, but test coverage changes, so it needs review |
| 1.7 | #26 | Sort `scripts/` into `audit/ benchmark/ capture/ diagnose/` and update `package.json` and CI paths. Delete one-off scripts whose findings are recorded in `validation/` | Grep for every script path first |

The review expects 1.3, 1.4 and 1.6 together to shrink `lib.rs` from about 23k lines to roughly 7–9k.

## Phase 2: Engine structure (refactor only)

Order matters. Each PR moves code, keeps it compiling, and passes the
gates unchanged.

1. **2.1 Module tree (#5)**: create `core/ world/ infrastructure/ population/
   transit/ persistence/ wasm/` and move files with `git mv` plus
   `pub(crate) use` re-exports, so call sites are unchanged. One domain per PR,
   starting with `wasm/`, which is just the `mod wasm` block at `lib.rs:~12450`,
   then `persistence/`.
2. **2.2 Enums (#10)**: replace the `kind`/`stage` strings with enums and
   `#[serde(rename_all = "snake_case")]`, so JSON, saves and `types.ts` stay
   the same. Do this before the WorldState split so the moved code is already
   typed.
3. **2.3 Typed IDs (#9)**: add `RoadId(u32)` and the other ID types with
   `#[serde(transparent)]`, keeping the wire format. Add generation counters
   in a **separate** PR with a save-version bump, since they change saves and
   the wire format.
4. **2.4 WorldState split (#6)**: create `NetworkState`, `ParcelState`,
   `BuildingState`, `PopulationState`, `TransitState` and the other sub-states.
   Add `#[serde(flatten)]` so the save shape does not change. Move one domain
   per PR. Fields become `pub(in crate::<domain>)`.
5. **2.5 Domain modules (#7)**: convert each `impl Engine` extension file into
   functions over `(&mut OwnState, &WorldView)`. Start with the files that
   already touch the fewest fields: `rail_platform`, `road_split`,
   `rear_extensions`. `WorldView` is a read-only borrow bundle of the other
   sub-states.
6. **2.6 Workspace (#8)**: create `crates/geometry`, `crates/masterplan` (with
   `blocks`, `frontage`, `development`), `crates/sim` and `crates/wasm`. First
   verify that the pure modules don't import `Engine` or the RNG. The wasm
   crate replaces `engine/` as the `npm run wasm` target, so update
   `build-wasm.mjs`.
7. **2.7 CODEOWNERS (#28)**: one entry per crate or domain folder. Skip this
   while there is only one maintainer, and add it when a second person joins.

## Phase 3: Simulation model (after 2.4–2.5)

| PR | Items | Kind | Notes |
|---|---|---|---|
| 3.1 | #16 | Refactor | Routing service: keep the adjacency structure and invalidate it on network revision. Add a revision-keyed route cache. Must pass `route_equivalence_tests.rs` unchanged. **Can start early**: it only needs the network sub-state, and it is the biggest performance win. The per-tick request cap is a behavior change, so it goes in a separate PR |
| 3.2 | #11 | Refactor | `System` trait (`cadence`, `offset`, `phase`, `run(&mut Ctx)`) plus a static registry that reproduces today's `step_one` order exactly. Replay must not change. Read/write sets are declared as documentation now and checked in debug builds later |
| 3.3 | #13 | Behavior | Queue structural changes and commit them once per tick, bumping the graph revision. Re-baseline replays |
| 3.4 | #14 | Behavior | Bounded event queue. First two flows: building completed → occupancy, business closed → reassign trips |
| 3.5 | #12 | Behavior | Bucket the `tick % N` work by entity hash. Measure tick-time variance before and after with the existing benchmark scripts |
| 3.6 | #15 | Mixed | Move building, road and vehicle definitions into versioned data (JSON next to `shared/expressway-profiles.json`), validated at startup. Do one category per PR: buildings first, then roads, then vehicles |

## Phase 4: Contract and web

The web items #20–#24 do not depend on the engine phases and can start right
after 1.2. Only #17–#19 wait on the engine.

| PR | Items | Depends on | Notes |
|---|---|---|---|
| 4.1 | #24, #29 | 1.2 | dependency-cruiser rules (render ↛ worker/storage/ui; ui ↛ render internals). Initial exception: render → camera/audio, removed by 4.3. Add a file-size warning over 1,500 lines (Rust and TS). Wire both into CI |
| 4.2 | #20 | 1.2 | `RenderLayer` interface (`init`, `update(slice, ctx)`, `frame(dt)`, `dispose`). Extract one layer per PR: wiring/overhead-lines first (newest and smallest), then parks, trees, water, roads, buildings, actors |
| 4.3 | #21 | 4.2 | App composition root in `main.tsx` or `app/`, plus a `PresentationState` that the renderer, camera director and audio director share. Remove the 6 camera/audio imports from `city.ts`. Drop the 4.1 exception |
| 4.4 | #22 | 1.2 | Split `kit.ts` by building family, and `director.ts` by mode (watch, follow, manual, review) |
| 4.5 | #23 | — | Move `asset-review.ts`, review recording and `?midrise=` behind a dev-only entry (`import.meta.env.DEV` dynamic import), and check the bundle shrinks |
| 4.6 | #17 | 2.2, 2.3 | `ts-rs` derive on the snapshot types, generating `web/src/generated/`. `types.ts` re-exports the generated types. CI's existing `git diff --exit-code` catches drift once the generator runs in `npm run wasm` |
| 4.7 | #19 | 2.1 | Movement layout constants defined once in Rust and emitted into the generated TS. A test asserts the engine and renderer strides match |
| 4.8 | #18 | 2.4, 4.6 | Snapshot slices with per-slice revisions; created/updated/removed deltas; full resync on a gap. Bump the worker protocol version. Ship behind the existing epoch/revision checks. Biggest web change, so do it last |

## Repository

- **#25**: move bulk evidence (`*.txt`, `*.json`, captures) to a dated release
  asset, using `hydrate-review-evidence.py` as the restore path. Keep the
  `.md` summaries and decisions. First check which tests or scripts read
  `validation/` files directly (`grep -rn "validation/" scripts tests engine`),
  and keep those files. This only fixes the size of new clones. Shrinking
  existing clones needs a history rewrite, which is a separate decision for
  the owner.

## Suggested order

```
Step 0 ─► 1.1, 1.2 ─► 1.3 ─► 1.4 ─► 1.5 ─► 2.1 ─► 2.2 ─► 2.3 ─► 2.4 ─► 2.5 ─► 2.6
                │                    └─(acceptance)─► 1.6
                ├─► 4.1 ─► 4.2 ─► 4.3      (web track, in parallel)
                ├─► 4.4, 4.5
                └─► 1.7, #25
2.4 ─► 3.1 ─► 3.2 ─► 3.3 ─► 3.4 ─► 3.5 ─► 3.6
2.3 ─► 4.6 ─► 4.7 ─► 4.8
```

## Deliberate departures from the review

- **Phase 4 web work does not wait for Phase 2.** Items #20–#24 never touch
  the engine.
- **Routing (#16) is pulled forward.** It needs only the network sub-state and
  can be checked against the existing equivalence tests.
- **Generation counters are split from typed IDs**, because only the counters
  change the save and wire format.
- **CODEOWNERS (#28) waits for a second contributor.**
- **Tests move to child modules, not `engine/tests/`.** Integration tests
  would force private engine items public.

## Decisions (2026-10-16)

1. Master-plan acceptance does **not** pass, so do 1.5 (feature-gate legacy), not 1.6.
2. Mechanical formatting lands now.
3. #25: untrack bulk evidence; no history rewrite unless the owner asks for one.
4. Save and wire compatibility need not be preserved. Do every disruptive change
   (formatting, module moves, WorldState split, typed IDs with generations, enums,
   workspace) before parallel work starts. Same-version save round-trip and replay
   determinism stay required; fixtures are regenerated when the format changes.

## Progress on `refactor/architecture`

Every refactor step was checked against main with byte-level WASM parity (save content,
snapshots and movement buffers through 120k ticks on seeds 87231, 5 and 0), the full
engine and web suites, and a WebGL smoke capture for renderer changes.

| Item | Status |
|---|---|
| Step 0: AGENTS.md, STATUS.md, dated notes out of specs/README | Done |
| #4: rustfmt, Prettier, format check in CI | Done |
| #3: tests out of lib.rs | Done (`src/tests/`) |
| #2: experiments out of production | Done (`src/experiments/`, one `ExperimentState` field) |
| #1: legacy planners | Behind the `legacy` feature; production builds exclude them; restore rejects saves without a master plan. Deletion waits on master-plan acceptance. |
| #26: scripts sorted | Done |
| #25: bulk evidence untracked | Done; kept at tag `evidence-2026-10-16` (local, not pushed). No history rewrite. |
| #5: domain module tree | Done; `lib.rs` is about 160 lines |
| #6: WorldState sub-states | Done (`network`, `land`, `construction`, `people`, `transit`, `legacy`), flattened into saves. Fields are `pub(crate)`; narrowing them to owner-only APIs is open. |
| #7: domain modules over sub-states | Partial: code lives in domain files, still as `impl Engine` |
| #8: Cargo workspace | Done: `city-plan` (pure), engine, `city-wasm` (bindings) |
| #10: enums for kinds and stages | Done, including traveler/resident activity, routine and transfer |
| #9: typed IDs | **Open.** Changing `Road.id` alone gives 153 compile errors that cascade. Generation counters are unnecessary because IDs come from one monotonic counter and are never reused. Adopt one ID type at a time with a `PartialEq<u32>` bridge. |
| #11: system registry | Done (`core/systems.rs`), with replays unchanged |
| #12–#15: bucketing, structural commit point, event queue, data definitions | Open: behavior changes; each needs a replay re-baseline |
| #16: routing service | Open |
| #17: generated TS types | Done (`npm run types`, CI drift check) |
| #18: snapshot slices and deltas | Open: protocol change |
| #19: wire layout from one definition | Done (`core/wire.rs`) |
| #20: render layers | Done: roads, landscape and actors layers, plus a `RenderLayer` registry with the first two registered layers. `city.ts` is 1.4k lines. |
| #21: camera/audio out of the renderer | Done: `CameraRig` injected by `main.tsx`; audio scene in `app/`; optics moved to `optics/` |
| #22: kit.ts and director.ts splits | kit.ts split by family; director.ts lost anchors and framing (1.75k lines, mode split open) |
| #23: dev-only review tooling | Architecture review route is dev-only and absent from production builds |
| #24, #29: import lint, file-size check | Done in CI |
| #27: ARCHITECTURE.md, CONTRIBUTING.md | Done |
| #28: CODEOWNERS | Deferred until a second contributor |

Open items are tracked as issues #25–#35 (typed IDs #25, owner-only state #26, director split #27, bucketing #28, commit point #29, event queue #30, data definitions #31, routing #32, snapshot deltas #33, CODEOWNERS #34, oversized files #35) and #4 (legacy deletion).
