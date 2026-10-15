# Runtime review checkpoint

This replaces the oversized PR12 review surface with source, specs and small
fixtures on a branch based on main `fca58ca12ec8ad8fe10546b22004f883a2ea57e2`.
It does not merge the earlier evidence-heavy branch or rewrite its history.

The runtime set combines engine, web and regression changes with main's parks,
signage and textured four-car fleet. Generation, infrastructure presentation,
train placement and platform geometry are integrated; the broader city fidelity
milestones remain incomplete. Passing tests do not establish visual acceptance.

## Evidence and test setup

Large fixtures and visual evidence are external. The matching `test-fixtures.tar.gz` and `test-fixtures-manifest.json` must be
obtained before fixture-based tests. This 23-file archive is 3.6 MB; the 336 MB
historical visual-evidence bundle is not required to compile/run these tests. Durable hosting is still
pending; this checkpoint is not ready to merge until the download is available.
The manifest marks native validation witnesses as test fixtures as well as the
large files under tests/fixtures.

```sh
python3 scripts/hydrate-review-evidence.py --archive /path/to/test-fixtures.tar.gz \
  --manifest /path/to/test-fixtures-manifest.json --destination . --fixtures-only --apply
npm ci
npm run wasm
npm run build
npm test
npm run test:engine
```

Omit --apply for verification without writes. Conflicting local files are refused.
Hydrated files are ignored. Two previously tracked bundle-owned files are removed
from this branch's index; their old blobs already on main are not removed from
history. Do not add hydrated data to the index with force.

## Validation and limitations

Reconstruction over the pinned main revision matched every exported source hash.
All 210 engine/web/test files in this review branch match that reconciled export.
The reconstructed source builds WASM and the production web application, and
native tests compile after hydration. Existing Rust warnings and the web bundle
size warning remain. These checks used local dependency caches on macOS.

The full reconstructed web run initially passed 639 tests and failed 27. Test-only
repairs addressed constructor-bypassing park mocks, a queue assertion using a
terminal chord instead of the platform tangent, and an infill assertion that
froze unrelated road growth. All 32 tests in the four affected files then passed;
the other 118 files passed before these changes. No second clean full run claimed.

The original working branch passed 500 native tests with 59 ignored. This is not a
substitute for the reconciled four-car branch's full suite. That integrated suite
now passes 504 tests, with 0 failures and 59 ignored (575.98 seconds on this macOS host).
One ignored test is explicitly a known failing, rejected opening-time treatment;
it was not fixed. Its assertions remain runnable as a historical diagnostic.

The 47 referenced Markdown documents are included, bringing this review to 272
changed paths. Historical exploratory tooling remains separate. Linked binary
evidence still requires external hosting; the two locked concept images and
the everyday-study prompt manifest are packaged in the verified reference supplement.
The historical publication archive has 37 transcripts redacted for private paths;
all 1,280 members verify against its refreshed manifest. Durable hosting is pending. Do not claim all referenced evidence is
published yet. Browser motion, platform-side boarding visibility, resource
stability and matched multi-seed visual acceptance remain outstanding.

## Published visual targets

The [locked concepts](https://github.com/jeremyung/thecity/releases/tag/locked-concepts-2026-10-14) are publicly accessible, with hashes verified against the original two images. Only those images and their manifest were published. Test fixtures, logs, prompt supplement and historical archive remain local; fixture download availability remains a merge prerequisite.
