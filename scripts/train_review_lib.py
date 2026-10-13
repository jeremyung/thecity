"""Shared matting and proportion measurement for the train review scripts.

These are the parts of `build-vehicle-kits.py` that a 20 m train forces to
change: a separable row+column background model instead of a per-row test (the
backdrop has a horizon and a horizontal gradient), and height measured to the
tyre contact line rather than the floor-shadow bounding box. The pilot found
both; Phase 2 of `specs/train-kit-plan.md` extracts them into the production
car and train builders.
"""
import numpy as np


def shift(mask, dy, dx):
    out = np.zeros_like(mask)
    h, w = mask.shape
    out[max(dy, 0):h + min(dy, 0), max(dx, 0):w + min(dx, 0)] = mask[max(-dy, 0):h + min(-dy, 0), max(-dx, 0):w + min(-dx, 0)]
    return out


def dilate(mask, r=1):
    out = mask.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dy * dy <= r * r:
                out |= shift(mask, dy, dx)
    return out


def erode(mask, r=1):
    return ~dilate(~mask, r)


def flood(seed, allowed):
    region = seed & allowed
    while True:
        grown = dilate(region, 1) & allowed
        if grown.sum() == region.sum():
            return region
        for _ in range(8):
            grown = dilate(grown, 1) & allowed
        region = grown


def background_model(rgb):
    """Separable backdrop: a per-row level from the left/right edges, which
    follow the horizon between the backdrop and the brighter studio floor, plus
    a per-column horizontal gradient from the car-free top band."""
    h, w, _ = rgb.shape
    edges = np.concatenate([rgb[:, :12], rgb[:, -12:]], axis=1)
    row_bg = np.median(edges, axis=1)
    band = min(150, max(20, h // 5))
    col = np.median(rgb[:band] - row_bg[:band, None, :], axis=0)
    return row_bg[:, None, :] + col[None, :, :]


def foreground(rgb):
    h, w, _ = rgb.shape
    near = np.abs(rgb - background_model(rgb)).max(axis=2) < 14
    seed = np.zeros((h, w), bool)
    seed[0] = seed[-1] = seed[:, 0] = seed[:, -1] = True
    background = flood(seed, near)
    fg = ~background
    fg = dilate(erode(fg, 3), 3)
    ys, xs = np.nonzero(fg)
    cy, cx = int(np.median(ys)), int(np.median(xs))
    seed = np.zeros_like(fg)
    seed[cy, cx] = True
    if not fg[cy, cx]:
        d = (ys - cy) ** 2 + (xs - cx) ** 2
        i = int(np.argmin(d))
        seed[ys[i], xs[i]] = True
    return flood(seed, fg)


def measure(rgb, fg, length_m, height_m):
    """Body proportion to the tyre contact line, with thin roof equipment
    (pantographs) excluded from the body height on a wide-run test."""
    ys, xs = np.nonzero(fg)
    x0, x1, y0, y1 = int(xs.min()), int(xs.max()), int(ys.min()), int(ys.max())
    w_px = x1 - x0 + 1
    lum = rgb[..., 0] * .2126 + rgb[..., 1] * .7152 + rgb[..., 2] * .0722
    sat = rgb.max(axis=2) - rgb.min(axis=2)
    dark = (lum < 80) & fg & (sat < 50)
    rows = np.nonzero(dark[:, x0:x1 + 1].any(axis=1))[0]
    ground = int(rows.max()) if len(rows) else y1
    counts = fg[y0:ground + 1, x0:x1 + 1].sum(axis=1)
    wide = np.nonzero(counts > w_px * .4)[0]
    body_top = y0 + int(wide.min()) if len(wide) else y0
    expected = length_m / height_m
    ratio = w_px / (ground - y0 + 1)
    body_ratio = w_px / (ground - body_top + 1)
    return dict(px=[x0, y0, x1, ground], width_px=w_px, height_px=ground - y0 + 1,
                body_height_px=ground - body_top + 1, ratio=ratio, body_ratio=body_ratio,
                expected=expected, error_pct=(ratio / expected - 1) * 100,
                body_error_pct=(body_ratio / expected - 1) * 100,
                pantograph=bool(body_top > y0 + 8))
