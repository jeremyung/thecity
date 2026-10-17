# Train texture pilot — 2026-10-12

First execution of Phase 1 of the [train kit plan](../../specs/train-kit-plan.md).
The vehicle pilot used one generated side elevation per body; a train also needs
a real head-on cab view, so this pilot tests four views of one Japanese
commuter EMU: cab car side, intermediate side, cab front, and a motor car with a
pantograph. Generated locally with HunyuanImage-2.1 full through ComfyUI, so
seed sweeps cost nothing.

## Setup

- Model: `hunyuanimage2.1_bf16.safetensors`, 30 steps, cfg 3.5, euler, seed 2101.
- Orthographic suffix and negative prompt per `art-workshop/train-pilot.json`.
- Sizes: 2048×768 for side elevations, 1024×1280 for the head-on front.
- Cost: $0.00 (4 local runs; hosted was not used). Wall time 8–28 s per image.
- Ledger: `local-ledger.json`; raw metrics `review-metrics.json`.

## Result

| View | Body L/H | Expected | Body error | Stripe height | Window height | Verdict |
|---|---|---|---|---|---|---|
| cab-side | 5.02 | 5.51 | −8.9% | 0.44 | 0.66 | keep |
| intermediate-side | 5.34 | 5.51 | −3.1% | 0.41 | 0.66 | keep |
| cab-front | 0.80 | 0.81 | −1.6% | 0.39 | 0.66 | keep |
| motor-roof | 5.47 | 5.51 | −0.8% | 0.43 | 0.66 | keep |

Body L/H excludes thin roof equipment (the method drops a pantograph but keeps
roof air-conditioner boxes). For the front the full-silhouette ratio is the
meaningful one and matches within 1.6%; the body-only number (0.95) is inflated
because the front narrows at its roof spine.

Livery and layout are consistent across views: the waist stripe sits at
0.39–0.44 of body height and the window band at 0.66 in every view. At 128 px
the side reads as a commuter EMU; at 64 px the front reads as a cab with a dark
windscreen, destination box, headlamps and tail lamps (see `at-scale.png`).

Images: `hy21-full-ortho--*.png`, overview `contact-sheet.jpg`.

## Findings for the kit builder

1. **The front works.** Hunyuan full produces a symmetric, well-lit cab face
   with the right equipment, so a front elevation is a viable kit source. This
   was the pilot's main risk and it is cleared.
2. **The studio floor survives the orthographic suffix.** All four images carry
   a brighter lower half (the floor) with a soft horizon, plus a contact shadow.
   The vehicle builder's matting compares each row only to its left/right edges;
   that fails here because the backdrop also has a horizontal gradient and a
   horizon. A separable row+column background model from the car-free top band
   separates the cars cleanly (`review-train-pilot.py`); the kit builder will
   need that instead of the vehicle builder's per-row test.
3. **Views are independent.** The "intermediate trailer" came back with a
   pantograph, and the cab front has a pantograph behind it. Text-to-image does
   not enforce a consist; the kit must assign roof equipment by role or the
   prompt must be tightened.
4. **Camera is still slightly high.** The roof plane is faintly visible and the
   wheels show a little perspective, as the vehicle pilot also saw. Acceptable
   at miniature scale; the traced outline only needs the side silhouette.
5. **Matting quality is the limit, not typology.** The car bodies are clean and
   easy to separate once the floor is modelled; logos and text are absent.

## Decision

Proceed to Phase 2 with HunyuanImage-2.1 full and these four views as the kit
sources for one commuter class. Carry forward two requirements:

- The kit builder uses a separable/2D background model, not the vehicle
  builder's per-row model (`build-vehicle-kits.py`).
- Roof equipment and the presence of a cab are assigned per consist role, and
  the front is traced as an end cap rather than approximated from the side.

Fallback if the livery or front needs to match an edited side exactly: use the
installed `Qwen-Image-2.1` edit model conditioned on the approved side, and
reach for `qwen/qwen-image-3` only if a front needs a cleaner flat elevation.

## Running locally

```sh
# Stop vLLM first to free the GPU; ComfyUI needs
# ~/Development/ComfyUI/extra_model_paths.yaml pointing at
# ~/Models/HunyuanImage-2.1/split_files.
cd ~/Development/ComfyUI && venv/bin/python main.py --listen 127.0.0.1 --port 8188 --disable-auto-launch &
python3 scripts/assets/generate-train-pilot-local.py hunyuanimage2.1_bf16.safetensors 30 3.5 euler hy21-full-ortho
python3 scripts/assets/review-train-pilot.py
```

## What is committed

The four elevations the kit reads, `local-ledger.json`, `review-metrics.json`,
`contact-sheet.jpg`, `at-scale.png`, the approved config
`../train-pilot.json`, and the local generation and review scripts.
