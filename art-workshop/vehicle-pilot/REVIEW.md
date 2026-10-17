# Vehicle texture pilot

Side-elevation source images for five authored vehicle kits, generated from one shared prompt set (`../vehicle-pilot.json`). Compares local HunyuanImage-2.1 (ComfyUI, RTX PRO 6000) with hosted models through OpenRouter. `contact-sheet.jpg` shows every result.

| Source | Time / image | Cost | Length:height error (mean) | Notes |
|---|---|---|---|---|
| Hunyuan 2.1 distilled, 8 steps | 3 s | $0 | 15% | Die-cast product photo: slight high angle, floor shadow, depth of field. Van faced right. |
| Hunyuan 2.1 full, 30 steps, cfg 3.5 | 12 s | $0 | 5% | Better proportions; box truck got a third axle. |
| Hunyuan 2.1 full + orthographic prompt suffix | 13 s | $0 | not measured | Floor, depth of field and extra axle gone; faint backdrop gradient and slight high angle remain. Tactile miniature finish suits Little Tokyo. |
| tencent/hy-image-v3.5-preview | 12–21 s | $0.024 | 8% | True orthographic and crisp. Taxi read as a kei van (−26%); hatch background dark and grainy. |
| qwen/qwen-image-3 | 48–59 s | $0.03 | 3% | Cleanest flat elevations and backgrounds, easiest to matte; slightly CG-plain. Best taxi. |
| google/gemini-nano-banana-2.1 | — | $0 | — | Provider quota 429; not charged, not retried. |

Proportion error is a background-difference bounding box; shadows inflate Hunyuan figures slightly.

Hosted spend: $0.27 of the $2 cap.

## Decision

Local Hunyuan 2.1 (full model, orthographic suffix) is not worse than the hosted models and costs nothing per image, so seed sweeps are free. Use it as the default source; use Qwen-Image-3 as the fallback when a type needs a cleaner flat elevation. Hunyuan 2.1 is text-to-image only, so front, rear and roof views will not exactly match the side; an editing model (e.g. Qwen-Image-2.1) is the upgrade path if that mismatch shows at street-camera scale.

Not yet done: matting, atlas packing, paint mask, kit geometry, in-city review.

## Running locally

ComfyUI (`~/Development/ComfyUI`, venv now has torch 2.14.1+cu130) needs `extra_model_paths.yaml` pointing at `~/Models/HunyuanImage-2.1/split_files`. The GPU must be free: stop vLLM first, and restart it with `~/vllm-venv/bin` on PATH (FlashInfer needs `ninja`).

```
python3 scripts/assets/generate-vehicle-pilot-local.py hunyuanimage2.1_bf16.safetensors 30 3.5 euler <tag> [vehicle ...]
```
The `hy21-full-ortho` run used:

- `PROMPT_SUFFIX`: "Technical vehicle elevation drawing rendered as a photograph-quality flat texture: camera exactly level with the axles, roof surface and tyre treads not visible, entire vehicle in sharp focus edge to edge, no depth of field, no floor and no shadow under the car, seamless uniform pale gray backdrop."
- `NEGATIVE_EXTRA`: "depth of field, bokeh, blurred background, floor, studio floor, ground shadow, elevated camera, roof visible, toy, diecast model, three axles, facing right, "

## What is committed

Only the side elevations that `scripts/assets/build-vehicle-kits.py` reads (listed in `art-workshop/vehicle-kit.json`) are kept in the repository, with both ledgers and the contact sheet. Every other generation is recorded in `ledger.json` and `local-ledger.json` with its prompt, model and cost, and can be regenerated from the configs in `art-workshop/vehicle-*.json`.
