// Development review of procedural architecture (?review=architecture, dev server only).
import React, { useEffect, useRef, useState } from 'react';
import { ArchitectureAssetReview } from '../render/asset-review';
import type { Style } from '../types';

export default function ArchitectureReviewRoute() {
  const materialCandidate =
    new URLSearchParams(window.location.search).get('material') === 'concrete'
      ? 'concrete'
      : 'plaster';
  const mount = useRef<HTMLDivElement>(null),
    viewer = useRef<ArchitectureAssetReview | null>(null);
  const [style, setStyle] = useState<Style>('anime'),
    [error, setError] = useState<string | null>(null),
    [materialStatus, setMaterialStatus] = useState<'loading' | 'ready' | 'error'>('loading'),
    [shopPlaster, setShopPlaster] = useState(true),
    [nightReview, setNightReview] = useState(false);
  const [lowriseStage, setLowriseStage] = useState<'open' | 'construction' | 'renovation'>('open'),
    [lowriseProgress, setLowriseProgress] = useState(0.5);
  useEffect(() => {
    if (!mount.current) return;
    let disposed = false;
    const review = new ArchitectureAssetReview(mount.current, style, materialCandidate);
    review.onMaterialStatus = setMaterialStatus;
    viewer.current = review;
    void review.start().catch((e) => {
      if (!disposed) setError(e instanceof Error ? e.message : String(e));
    });
    return () => {
      disposed = true;
      review.dispose();
      viewer.current = null;
    };
  }, []);
  useEffect(() => {
    viewer.current?.setStyle(style);
  }, [style]);
  useEffect(() => {
    viewer.current?.setLowriseStage(lowriseStage, lowriseProgress);
  }, [lowriseStage, lowriseProgress]);
  return (
    <main className="architecture-review">
      <style>{`
      .architecture-review{position:fixed;inset:0;overflow:hidden;background:#afc4ca;color:#263d43;font-family:ui-sans-serif,system-ui,-apple-system,sans-serif}
      .architecture-review__canvas{position:absolute;inset:0;touch-action:none}
      .architecture-review__canvas canvas{display:block;width:100%;height:100%;outline:none}
      .architecture-review__head{position:absolute;top:0;left:0;right:0;display:flex;align-items:flex-start;justify-content:space-between;gap:20px;padding:calc(env(safe-area-inset-top) + 18px) max(20px,env(safe-area-inset-right)) 12px max(20px,env(safe-area-inset-left));pointer-events:none;background:linear-gradient(#afc4cae8,transparent)}
      .architecture-review__title{margin:0;font-family:Georgia,'Hiragino Mincho ProN',serif;font-weight:400;font-size:clamp(24px,3vw,38px);letter-spacing:-.035em;line-height:1.02}
      .architecture-review__kicker{display:block;margin-bottom:9px;font-size:10px;letter-spacing:.19em;text-transform:uppercase;color:#a45442}
      .architecture-review__actions{display:flex;align-items:center;gap:7px;pointer-events:auto;flex-wrap:wrap;justify-content:flex-end}
      .architecture-review button,.architecture-review a{min-height:44px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 14px;font:inherit;font-size:12px;text-decoration:none;display:inline-flex;align-items:center;justify-content:center;cursor:pointer;backdrop-filter:blur(8px)}
      .architecture-review button[aria-pressed="true"]{background:#344f50;color:#f6eedf;border-color:#344f50}
      .architecture-review button:focus-visible,.architecture-review a:focus-visible{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__toggle{min-height:44px;display:inline-flex;align-items:center;gap:8px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 12px;font-size:12px;cursor:pointer;backdrop-filter:blur(8px)}
      .architecture-review__toggle input{width:16px;height:16px;margin:0;accent-color:#a45442}
      .architecture-review__toggle:focus-within{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__stage{min-height:44px;display:inline-flex;align-items:center;gap:7px;border:1px solid #566b6b80;border-radius:3px;background:#f5f0e7e8;color:#293f43;padding:0 9px;font-size:11px;backdrop-filter:blur(8px)}
      .architecture-review__stage select{max-width:116px;border:0;background:transparent;color:inherit;font:inherit;font-size:11px}
      .architecture-review__stage input[type=range]{width:78px;margin:0;accent-color:#a45442}
      .architecture-review__stage output{min-width:30px;font-variant-numeric:tabular-nums}
      .architecture-review__stage:focus-within{outline:2px solid #b65d4b;outline-offset:2px}
      .architecture-review__hint{position:absolute;right:22px;top:92px;max-width:230px;text-align:right;font-size:11px;line-height:1.5;color:#30484d;text-shadow:0 1px 8px #d8e3e1}
      .architecture-review__samples{position:absolute;left:max(20px,env(safe-area-inset-left));bottom:calc(env(safe-area-inset-bottom) + 20px);display:flex;gap:0;flex-wrap:wrap;max-width:min(920px,calc(100vw - 40px));background:#f6f1e8e8;box-shadow:0 12px 35px #23363824;backdrop-filter:blur(12px);border:1px solid #fff9;border-radius:4px;overflow:hidden}
      .architecture-review__sample{padding:12px 16px 11px;border-right:1px solid #7b858033;min-width:142px}
      .architecture-review__sample:last-child{border-right:0}
      .architecture-review__sample b{display:block;font-size:11px;font-weight:550;letter-spacing:.02em}
      .architecture-review__sample small{display:block;margin-top:4px;font-size:10px;color:#607171;letter-spacing:.08em;text-transform:uppercase}
      .architecture-review__error{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);padding:16px 20px;background:#f6eee8;color:#7c332e;border:1px solid #be7768;max-width:min(90vw,520px)}
      @media(max-width:640px){.architecture-review__head{gap:10px;padding-top:calc(env(safe-area-inset-top) + 12px);padding-left:max(12px,env(safe-area-inset-left));padding-right:max(12px,env(safe-area-inset-right))}.architecture-review__title{font-size:25px}.architecture-review__actions{gap:4px}.architecture-review button,.architecture-review a{padding:0 10px;font-size:11px}.architecture-review__toggle{min-height:44px;padding:0 9px;font-size:11px}.architecture-review__stage{min-height:44px;padding:0 7px;gap:5px}.architecture-review__stage select{max-width:100px}.architecture-review__stage input[type=range]{width:66px}.architecture-review__hint{top:84px;right:12px;max-width:180px;font-size:10px}.architecture-review__samples{left:12px;right:12px;bottom:calc(env(safe-area-inset-bottom) + 12px);max-width:none}.architecture-review__sample{flex:1;min-width:calc(50% - 1px);padding:9px 10px;border-bottom:1px solid #7b858033}.architecture-review__sample:nth-child(even){border-right:0}.architecture-review__sample b{font-size:10px}.architecture-review__sample small{font-size:9px}}
      @media(prefers-reduced-motion:reduce){.architecture-review *{scroll-behavior:auto!important;transition:none!important}}
    `}</style>
      <div
        className="architecture-review__canvas"
        ref={mount}
        aria-label="Interactive architectural specimen and station street. Drag to orbit; scroll to zoom."
      />
      <header className="architecture-review__head">
        <div>
          <span className="architecture-review__kicker">
            Development review · procedural architecture
          </span>
          <h1 className="architecture-review__title">Last Train, First Light</h1>
        </div>
        <div className="architecture-review__actions" aria-label="Architecture review controls">
          <button type="button" aria-pressed={style === 'anime'} onClick={() => setStyle('anime')}>
            Anime
          </button>
          <button
            type="button"
            aria-pressed={style === 'miniature'}
            onClick={() => setStyle('miniature')}
          >
            Miniature
          </button>
          <label className="architecture-review__stage">
            <span>Low-rise stage</span>
            <select
              aria-label="Low-rise specimen stage"
              value={lowriseStage}
              onChange={(event) =>
                setLowriseStage(event.target.value as 'open' | 'construction' | 'renovation')
              }
            >
              <option value="open">Open</option>
              <option value="construction">Construction</option>
              <option value="renovation">Renovation</option>
            </select>
          </label>
          {lowriseStage === 'construction' && (
            <label className="architecture-review__stage">
              <span>Progress</span>
              <input
                aria-label="Low-rise construction progress"
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={lowriseProgress}
                onChange={(event) => setLowriseProgress(Number(event.target.value))}
              />
              <output>{Math.round(lowriseProgress * 100)}%</output>
            </label>
          )}
          <label className="architecture-review__toggle">
            <input
              type="checkbox"
              checked={shopPlaster}
              onChange={(event) => {
                setShopPlaster(event.target.checked);
                viewer.current?.setShopPlaster(event.target.checked);
              }}
            />
            Candidate on shop
          </label>
          <label className="architecture-review__toggle">
            <input
              type="checkbox"
              checked={nightReview}
              onChange={(event) => {
                setNightReview(event.target.checked);
                viewer.current?.setNight(event.target.checked);
              }}
            />
            Night lighting
          </label>
          <button type="button" onClick={() => viewer.current?.resetCamera()}>
            Reset view
          </button>
          <button type="button" onClick={() => viewer.current?.inspectMaterial()}>
            Inspect material
          </button>
          <button type="button" onClick={() => viewer.current?.inspectShop()}>
            Inspect shop
          </button>
          <button type="button" onClick={() => viewer.current?.inspectApartment()}>
            Inspect apartment
          </button>
          <button type="button" onClick={() => viewer.current?.inspectOffice()}>
            Inspect office
          </button>
          <button type="button" onClick={() => viewer.current?.inspectLowrise()}>
            Inspect low-rise street
          </button>
          <button type="button" onClick={() => viewer.current?.inspectLowrise(true)}>
            Inspect rear service
          </button>
          <a href="/" aria-label="Return to the living city">
            Return to city
          </a>
        </div>
      </header>
      <p className="architecture-review__hint">
        Drag to orbit · scroll to move closer
        <br />
        Shared building kits · controlled material lighting
      </p>
      <section className="architecture-review__samples" aria-label="Architecture specimens">
        <div className="architecture-review__sample">
          <b>Kissa Aoba</b>
          <small>Corner shop · 2 floors</small>
        </div>
        <div className="architecture-review__sample">
          <b>Apartment tower</b>
          <small>Individual bays · 20 floors</small>
        </div>
        <div className="architecture-review__sample">
          <b>Office block</b>
          <small>Street frontage · 8 floors</small>
        </div>
        <div className="architecture-review__sample">
          <b>Mixed-use tower</b>
          <small>Podium · 28 floors</small>
        </div>
        <div className="architecture-review__sample">
          <b>Aobagawa station</b>
          <small>Platform · canopy · rail</small>
        </div>
        <div className="architecture-review__sample">
          <b>Material study · candidate</b>
          <small>
            {materialCandidate === 'concrete' ? 'Pilot concrete · 512 px' : 'Ivory plaster'} · 2 m
            tiles · 6 × 6 m ·{' '}
            {materialStatus === 'ready'
              ? 'loaded'
              : materialStatus === 'error'
                ? 'load failed'
                : 'loading'}
          </small>
        </div>
      </section>
      {error && (
        <p className="architecture-review__error" role="alert">
          The architecture review could not start: {error}
        </p>
      )}
    </main>
  );
}
