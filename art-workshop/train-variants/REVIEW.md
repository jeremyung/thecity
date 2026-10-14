# Train class and generator-model sweep — 2026-10-13

Second pilot, run after the question "what about Shinkansen, subway/JR, etc.?"
Two independent tests:

1. **Class sweep** — can local HunyuanImage-2.1 draw the other train classes?
   Four classes × side + head-on front, `art-workshop/train-classes.json`.
2. **Generator comparison** — the commuter cab through full bf16, fp8 and the
   distilled 8-step model, same prompts and seed.

Generation only; none of these classes has infrastructure in the prototype yet.
This informs later milestones, not runtime scope.

## Setup

- Model: `hunyuanimage2.1_bf16` full, 30 steps, cfg 3.5, euler, seed 3101
  (classes) / 2101 (models). Cost $0; wall time 1–26 s per image.
- Ledgers: `local-ledger.json`; metrics `review-metrics.json`,
  `model-metrics.json`.

## Class sweep result

| View | Body L/H | Expected | Error | Front symmetry | Sharpness | Verdict |
|---|---|---|---|---|---|---|
| shinkansen-side | 6.49 | 6.85 | −5.3% | — | 400 | keep |
| shinkansen-front | 1.00 | 0.92 | +9.2% | 0.81 | 206 | keep (head-on) |
| subway-side | 4.84 | 5.14 | −5.9% | — | 338 | keep |
| subway-front | 1.02 | 0.80 | +27.7% | 0.87 | 173 | keep (head-on) |
| express-side | 5.02 | 5.83 | −14.0% | — | 317 | keep |
| express-front | 1.35 | 0.81 | +68.1% | **0.58** | 267 | **reject: three-quarter view** |
| tram-side | 5.29 | 5.88 | −10.0% | — | 269 | keep |
| tram-front | 1.14 | 0.68 | +68.6% | 0.85 | 126 | borderline: mildly angled |

`class-sides.jpg` and `class-fronts.jpg` show the results. All four **side**
elevations are convincing and within 15% of the stated proportions, including
the Shinkansen's long nose.

The **fronts** split by nose shape. Boxy fronts came back genuinely head-on and
symmetric (Shinkansen 0.81, subway 0.87). The classes with a distinctive raked
or rounded nose — limited express and tram — came back as a flattering
three-quarter hero shot, exactly the failure the first pilot's front risk
warned about. The measured front ratio is the tell: a true head-on is near the
0.8–0.9 width/height ratio, while the two failures sit at 1.1–1.35 because side
content is visible. The symmetry metric catches the express (0.58) but only
mildly flags the tram (0.85), so ratio error is the more reliable detector.

## Generator comparison result

| Model | Side body L/H | Front symmetry | Front sharpness |
|---|---|---|---|
| full bf16, 30 steps | 5.02 (−8.9%) | 0.86 | 230 |
| fp8, 30 steps | 5.03 (−8.7%) | 0.87 | 213 |
| distilled bf16, 8 steps | 5.39 (−2.2%) | 0.75 | **88** |

`model-compare.jpg` shows the side-by-side. **fp8 is visually
indistinguishable from full bf16** on both views (sharpness 213 vs 230) and is
the better default because it halves VRAM. **Distilled 8-step is much faster but
visibly softer** (sharpness 88) and less reliable on the front (0.75, partly
three-quarter). Use it for fast silhouette exploration, not final sources.

## Qwen-Image-2.1: not usable here

The model is installed at `~/Models/Qwen-Image-2.1` as a diffusers pipeline
(`QwenImage21Pipeline`, needs diffusers `0.37.0.dev0`). It could not be run:

- ComfyUI's `DiffusersLoader` lists **zero** models, and the ComfyUI venv has no
  `diffusers` package installed.
- No Qwen-Image-**Edit** weights are present, so the front/side consistency test
  this was meant to answer is not possible locally.

So the only local generator available is HunyuanImage-2.1. Reaching for an edit
model needs a deliberate install (diffusers plus, likely, a Qwen-Image-Edit
checkpoint), which is its own small project.

## Conclusion

- The generator can produce **credible art for all four classes**, so Shinkansen
  and subway are blocked by world infrastructure, not by asset production.
- The **front is the fragile view, specifically for raked noses**. The kit plan
  should add: generate 8–16 seeds per front and keep only a near-head-on result
  (front ratio within ~10% of width/height and symmetry > 0.75), or bring in an
  edit/control model. Do not assume one front image is usable.
- **Use fp8 as the production model**; full bf16 is the reference; distilled is
  an exploration shortcut only.
- None of this changes the plan's class gating: the demonstrated classes are
  available when their corridors arrive.

## Files

`art-workshop/train-classes.json`; the eight class images; `class-sides.jpg`,
`class-fronts.jpg`; the three fp8/distilled commuter images; `model-compare.jpg`;
`local-ledger.json`, `review-metrics.json`, `model-metrics.json`; and the review
scripts `scripts/review-train-variants.py`, `scripts/train_review_lib.py`.
