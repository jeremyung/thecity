"""Review the exploratory train-class sweep and the generator-model comparison.

Class sweep:  art-workshop/train-classes.json  -> art-workshop/train-variants/hy21-full-ortho--*
Model compare: train-pilot.json commuter views -> hy21-full-ortho, hy21-fp8, hy21-distilled

Front-view symmetry is a simple automatic test for the pilot's observed failure:
the generator sometimes returns a flattering three-quarter "front". A true
head-on view is near-mirror-symmetric; a three-quarter view is not.

usage: scripts/review-train-variants.py
"""
import json, pathlib
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from train_review_lib import foreground, measure

ROOT = pathlib.Path(__file__).resolve().parent.parent
PILOT = ROOT / 'art-workshop/train-pilot'
VARIANTS = ROOT / 'art-workshop/train-variants'
CLASSES = json.loads((ROOT / 'art-workshop/train-classes.json').read_text())
TAG = 'hy21-full-ortho'


def font(size, bold=False):
    name = 'DejaVuSans-Bold.ttf' if bold else 'DejaVuSans.ttf'
    try: return ImageFont.truetype(f'/usr/share/fonts/truetype/dejavu/{name}', size)
    except OSError: return ImageFont.load_default()


def symmetry(fg):
    """IoU of the foreground with its horizontal mirror. Head-on fronts are
    high; three-quarter views are markedly lower."""
    m = fg[:, ::-1]
    union = (fg | m).sum()
    return float((fg & m).sum() / union) if union else 0.0


def sharpness(rgb, box):
    """Variance of a 3x3 Laplacian over the body crop: a cheap detail proxy."""
    x0, y0, x1, y1 = box
    g = (rgb[y0:y1 + 1, x0:x1 + 1] * [.2126, .7152, .0722]).sum(axis=2)
    if g.shape[0] < 3 or g.shape[1] < 3:
        return 0.0
    lap = (4 * g[1:-1, 1:-1] - g[:-2, 1:-1] - g[2:, 1:-1] - g[1:-1, :-2] - g[1:-1, 2:])
    return float(lap.var())


def analyze(path, view):
    rgb = np.asarray(Image.open(path).convert('RGB')).astype(np.float32)
    fg = foreground(rgb)
    dims = view['dimensionsM']
    length, height = (dims[0], dims[2]) if view['view'] == 'side' else (dims[0], dims[1])
    m = measure(rgb, fg, length, height)
    m['symmetry'] = symmetry(fg)
    m['sharpness'] = sharpness(rgb, m['px'])
    return rgb, fg, m


def label(im, text, out):
    d = ImageDraw.Draw(im)
    d.rectangle([0, 0, im.width, 24], fill='#f4f1ea')
    d.text((8, 4), text, fill='#2a3a4a', font=font(16))
    return im


def class_sheets(results):
    # Sides stacked, fronts in a row.
    sides = [(v, im, m) for v, im, m in results if v['view'] == 'side']
    fronts = [(v, im, m) for v, im, m in results if v['view'] == 'front']
    width = 900
    tiles = []
    for v, im, m in sides:
        t = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
        tiles.append(label(t, f"{v['id']}  body L/H {m['body_ratio']:.2f} vs {m['expected']:.2f} ({m['body_error_pct']:+.1f}%)", None))
    h = 30 + sum(t.height + 26 for t in tiles)
    sheet = Image.new('RGB', (width + 20, h), '#f4f1ea')
    y = 8
    for t in tiles:
        sheet.paste(t, (10, y)); y += t.height + 26
    sheet.save(VARIANTS / 'class-sides.jpg', quality=86)

    scale = 440 / max(im.height for _, im, _ in fronts)
    ftiles = [im.resize((round(im.width * scale), round(im.height * scale)), Image.LANCZOS) for _, im, _ in fronts]
    gap = 12
    fw = sum(t.width for t in ftiles) + gap * (len(ftiles) + 1)
    fh = max(t.height for t in ftiles) + 58
    fsheet = Image.new('RGB', (fw, fh), '#f4f1ea')
    d = ImageDraw.Draw(fsheet)
    x = gap
    for (v, _, m), t in zip(fronts, ftiles):
        fsheet.paste(t, (x, 40))
        d.text((x, 10), f"{v['id']}", fill='#2a3a4a', font=font(16, bold=True))
        d.text((x, 24), f"symmetry {m['symmetry']:.2f}  L/H {m['ratio']:.2f}", fill='#3a4a5a', font=font(14))
        x += t.width + gap
    fsheet.save(VARIANTS / 'class-fronts.jpg', quality=86)


def model_compare():
    rows = [('full bf16, 30 steps', PILOT / 'hy21-full-ortho--cab-side.png', PILOT / 'hy21-full-ortho--cab-front.png'),
            ('fp8, 30 steps', VARIANTS / 'hy21-fp8--cab-side.png', VARIANTS / 'hy21-fp8--cab-front.png'),
            ('distilled bf16, 8 steps', VARIANTS / 'hy21-distilled--cab-side.png', VARIANTS / 'hy21-distilled--cab-front.png')]
    side_view = next(v for v in json.loads((ROOT / 'art-workshop/train-pilot.json').read_text())['views'] if v['id'] == 'cab-side')
    front_view = next(v for v in json.loads((ROOT / 'art-workshop/train-pilot.json').read_text())['views'] if v['id'] == 'cab-front')
    metrics = []
    for name, side_path, front_path in rows:
        _, _, sm = analyze(side_path, side_view)
        _, _, fm = analyze(front_path, front_view)
        metrics.append(dict(model=name, side_body_ratio=sm['body_ratio'], side_error_pct=sm['body_error_pct'],
                            front_ratio=fm['ratio'], front_symmetry=fm['symmetry'], sharpness=fm['sharpness']))

    th = 260
    cols = []
    for name, side_path, front_path in rows:
        side = Image.open(side_path).convert('RGB')
        front = Image.open(front_path).convert('RGB')
        cols.append((name, side.resize((440, round(side.height * 440 / side.width)), Image.LANCZOS),
                     front.resize((th, round(front.height * th / front.width)), Image.LANCZOS)))
    gap = 16
    w = gap + sum(max(c[1].width + c[2].width + 20, 300) for c in cols) + gap * (len(cols) - 1)
    row_h = max(max(c[1].height, c[2].height) for c in cols) + 96
    sheet = Image.new('RGB', (w, row_h + 20), '#f4f1ea')
    d = ImageDraw.Draw(sheet)
    d.text((gap, 4), 'Model comparison on the commuter cab (same prompts, seed 2101)', fill='#23324f', font=font(20, bold=True))
    x = gap
    for (name, side, front), m in zip(cols, metrics):
        d.text((x, 40), name, fill='#2a3a4a', font=font(16, bold=True))
        d.text((x, 60), f"side L/H {m['side_body_ratio']:.2f} ({m['side_error_pct']:+.1f}%)", fill='#3a4a5a', font=font(14))
        d.text((x, 76), f"front sym {m['front_symmetry']:.2f}  sharp {m['sharpness']:.0f}", fill='#3a4a5a', font=font(14))
        y = 96
        sheet.paste(side, (x, y))
        sheet.paste(front, (x + side.width + 16, y))
        x += side.width + front.width + 20 + gap
    sheet.save(VARIANTS / 'model-compare.jpg', quality=88)
    (VARIANTS / 'model-metrics.json').write_text(json.dumps(metrics, indent=2) + '\n')
    return metrics


def main():
    results = []
    for view in CLASSES['views']:
        path = VARIANTS / f'{TAG}--{view["id"]}.png'
        rgb, fg, m = analyze(path, view)
        results.append((view, Image.open(path).convert('RGB'), m))

    print(f"{'class view':<22} {'body L/H':>9} {'expect':>7} {'err':>7} {'sym':>6} {'sharp':>8}")
    for v, _, m in results:
        print(f"{v['id']:<22} {m['body_ratio']:>9.2f} {m['expected']:>7.2f} {m['body_error_pct']:>6.1f}% "
              f"{m['symmetry']:>6.2f} {m['sharpness']:>8.0f}")
    class_sheets(results)
    print('wrote class-sides.jpg, class-fronts.jpg')

    metrics = model_compare()
    print('model comparison:')
    for m in metrics:
        print(f"  {m['model']:<26} side {m['side_body_ratio']:.2f} ({m['side_error_pct']:+.1f}%)  "
              f"front sym {m['front_symmetry']:.2f}  sharp {m['sharpness']:.0f}")
    print('wrote model-compare.jpg, model-metrics.json')

    (VARIANTS / 'review-metrics.json').write_text(json.dumps(
        [{**{k: v for k, v in view.items() if k in ('id', 'view', 'role', 'dimensionsM')},
          **{k: (round(float(x), 3) if isinstance(x, (int, float, np.floating)) else x) for k, x in m.items()}}
         for view, _, m in results], indent=2, default=lambda x: x.item() if hasattr(x, 'item') else x) + '\n')


if __name__ == '__main__':
    main()
