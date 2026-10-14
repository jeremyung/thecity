"""Measure the train pilot against train-pilot.json criteria and build the
contact sheet. Reuses the shared train matting library so the proportion
numbers match what the kit builder should trace.

usage: scripts/review-train-pilot.py
"""
import json, pathlib
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from train_review_lib import background_model, foreground, measure

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONFIG = json.loads((ROOT / 'art-workshop/train-pilot.json').read_text())
PILOT = ROOT / 'art-workshop/train-pilot'
TAG = 'hy21-full-ortho'
TEAL = np.array([0x1f, 0x6f, 0x6d])  # the requested livery stripe, for band detection


def analyze(view, rgb, fg):
    dims = view['dimensionsM']
    length, height = (dims[0], dims[2]) if view['view'] == 'side' else (dims[0], dims[1])
    m = measure(rgb, fg, length, height)
    x0, y0, x1, ground = m['px']
    h_px = m['height_px']
    # Waist stripe: teal pixels inside the body, as a fraction of body height
    # above the tyre contact. Also window band: dark pixels in the upper body.
    body = fg[y0:ground + 1, x0:x1 + 1]
    crop = rgb[y0:ground + 1, x0:x1 + 1]
    dist = np.sqrt(((crop - TEAL[None, None, :]) ** 2).sum(axis=2))
    stripe = body & (dist < 70)
    stripe_y = ((h_px - 1) - np.nonzero(stripe)[0].mean()) / h_px if stripe.sum() > 50 else None
    lum = crop[..., 0] * .2126 + crop[..., 1] * .7152 + crop[..., 2] * .0722
    rel = 1 - np.arange(h_px)[:, None] / h_px
    win = body & (lum < 70) & (rel > .5)
    win_y = ((h_px - 1) - np.nonzero(win)[0].mean()) / h_px if win.sum() > 50 else None
    # Backdrop variation the matte has to absorb: border residual of the fit.
    hh, ww, _ = rgb.shape
    mm = max(2, hh // 100)
    byy, bxx = np.mgrid[0:hh, 0:ww]
    bd = (byy < mm) | (byy >= hh - mm) | (bxx < mm) | (bxx >= ww - mm)
    bg_std = float(np.abs(rgb[bd] - background_model(rgb)[bd]).std())
    return {**m, 'stripe_y': stripe_y, 'window_y': win_y, 'bg_std': bg_std}


def at_scale():
    """Show what each elevation looks like at its on-screen width. The right
    copy is a smooth magnification of the left, so it reveals the detail that
    survives downsampling without nearest-neighbour pixel blocks."""
    picks = [('hy21-full-ortho--cab-side.png', 128, 'side · actual 128 px wide'),
             ('hy21-full-ortho--cab-front.png', 64, 'front · actual 64 px wide')]
    rows = []
    for name, width, label in picks:
        full = Image.open(PILOT / name).convert('RGB')
        small = full.resize((width, max(1, round(full.height * width / full.width))), Image.LANCZOS)
        zoom = max(2, min(6, round(260 / max(1, small.height))))
        rows.append((small, small.resize((small.width * zoom, small.height * zoom), Image.LANCZOS), label, zoom))

    pad, gap, text_h = 20, 26, 52
    height = pad + sum(text_h + r[1].height + gap for r in rows)
    sheet = Image.new('RGB', (560, height), '#f4f1ea')
    d = ImageDraw.Draw(sheet)
    try: font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 16)
    except OSError: font = ImageFont.load_default()
    try: big = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 20)
    except OSError: big = font
    d.text((pad, 12), 'Train pilot at miniature street scale', fill='#23324f', font=big)
    y = pad + text_h - 10
    for small, zoom_img, label, zoom in rows:
        d.text((pad, y), label, fill='#3a4a5a', font=font)
        d.text((pad, y + 20), f'right: smooth {zoom}× magnification (no extra detail)', fill='#7a8794', font=font)
        ty = y + text_h
        sheet.paste(small, (pad, ty))
        d.rectangle([pad - 1, ty - 1, pad + small.width, ty + small.height], outline='#c8c2b4')
        sheet.paste(zoom_img, (pad + small.width + gap, ty))
        y = ty + zoom_img.height + gap
    out = PILOT / 'at-scale.png'
    sheet.save(out)
    print('wrote', out.relative_to(ROOT))


def main():
    rows = []
    for view in CONFIG['views']:
        path = PILOT / f'{TAG}--{view["id"]}.png'
        rgb = np.asarray(Image.open(path).convert('RGB')).astype(np.float32)
        rows.append((view, analyze(view, rgb, foreground(rgb))))

    print(f"{'view':<20} {'L/H':>6} {'body L/H':>9} {'expected':>9} {'body err':>9} {'stripe y':>9} {'window y':>9} {'bg std':>7}")
    for view, m in rows:
        print(f"{view['id']:<20} {m['ratio']:>6.2f} {m['body_ratio']:>9.2f} {m['expected']:>9.2f} "
              f"{m['body_error_pct']:>8.1f}% "
              f"{(m['stripe_y'] if m['stripe_y'] is not None else float('nan')):>9.2f} "
              f"{(m['window_y'] if m['window_y'] is not None else float('nan')):>9.2f} {m['bg_std']:>7.2f}")

    tiles = []
    for view, m in rows:
        im = Image.open(PILOT / f'{TAG}--{view["id"]}.png').convert('RGB')
        scale = 900 / im.width
        tiles.append((view, im.resize((900, int(im.height * scale))), m))

    title = 46
    total_h = title + sum(t.height + 30 for _, t, _ in tiles)
    sheet = Image.new('RGB', (940, total_h), '#f4f1ea')
    d = ImageDraw.Draw(sheet)
    try: font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 18)
    except OSError: font = ImageFont.load_default()
    try: big = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 26)
    except OSError: big = font
    d.text((20, 10), 'Train texture pilot — HunyuanImage-2.1 full, orthographic suffix', fill='#23324f', font=big)
    y = title
    for view, tile, m in tiles:
        sheet.paste(tile, (20, y))
        label = (f"{view['id']}  ·  {view['role']}  ·  "
                 f"body L/H {m['body_ratio']:.2f} vs {m['expected']:.2f} ({m['body_error_pct']:+.1f}%)")
        d.text((20, y + tile.height + 4), label, fill='#3a4a5a', font=font)
        y += tile.height + 30
    out = PILOT / 'contact-sheet.jpg'
    sheet.save(out, quality=86)
    print('wrote', out.relative_to(ROOT))

    at_scale()

    (PILOT / 'review-metrics.json').write_text(json.dumps(
        [{**{k: v for k, v in view.items() if k in ('id', 'view', 'role', 'dimensionsM')},
          **{k: (round(float(x), 3) if isinstance(x, (int, float, np.floating)) else x) for k, x in m.items()}}
         for view, m in rows], indent=2, default=lambda x: x.item() if hasattr(x, 'item') else x) + '\n')


if __name__ == '__main__':
    main()
