# Initial repository checkpoint

This commit captures the runnable prototype, specifications, concept/reference art, offline texture pilot, and accumulated validation evidence. It is a work-in-progress checkpoint, not a visual-quality release.

Verified for this checkpoint:

- `npm test`: 50 files, 258 tests passed.
- `npm run test:engine -- --release`: 203 tests passed; 21 explicitly ignored diagnostics/experiments; no failures.
- `npm run build`: TypeScript and Vite passed; the existing large-main-bundle warning remains.
- Credential-pattern scan of candidate text files: no matches; local environment files are excluded.
- Compiled WASM collision comparison: exact save, snapshot, and movement equality across 300,000 ticks, plus equivalent resumed continuation. See `collision-wasm-replay-comparison.json`.

The ignored geometry-derived infill probe has not found an admissible branch for the studied mature gap. It remains diagnostic-only and is not a completed density feature. Visual fidelity, full browser performance acceptance, and live validation of the latest resource ownership changes remain incomplete. Historical test reports do not supersede these limitations.

Generated WASM bindings/binaries, build outputs, dependencies, local toolchains, and credentials are excluded. Follow the root README to install dependencies and regenerate WASM before running a fresh checkout.
