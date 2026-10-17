import { useEffect, useRef, useState } from 'react';
import * as THREE from 'three/webgpu';
import {
  positionLocal,
  normalLocal,
  triplanarTexture,
  texture,
  float,
  vec3,
  vec4,
  color,
  mix,
  smoothstep,
  pass,
  mrt,
  output,
  normalView,
  directionToColor,
  colorToDirection,
  sample,
} from 'three/tsl';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { ApartmentResources } from '../render/architecture/resources';
import { buildBlockStudy } from '../render/architecture/block-study';
import {
  buildResidentialTower,
  RESIDENTIAL_TOWERS,
  type ResidentialTowerRecipe,
  APARTMENT_RECIPE_IDS,
  APARTMENT_RECIPES,
  type ApartmentRecipeId,
} from '../render/architecture';
import { buildingKit, disposeGroup } from '../render/kit';
import type { Building } from '../types';
import { SkyEnvironment } from '../render/sky-environment';
import { ao } from 'three/addons/tsl/display/GTAONode.js';

export function ApartmentPilotRoute() {
  const initialTower = new URLSearchParams(location.search).get('family') === 'tower';
  const mount = useRef<HTMLDivElement>(null);
  const rebuild = useRef<(() => void) | null>(null);
  const view = useRef<((preset: 'building' | 'street' | 'rear') => void) | null>(null);
  const [width, setWidth] = useState(initialTower ? 26 : 12),
    [depth, setDepth] = useState(initialTower ? 24 : 14),
    [floors, setFloors] = useState(initialTower ? 32 : 7),
    [seed, setSeed] = useState(32);
  const [mode, setMode] = useState<'single' | 'block' | 'tower'>(initialTower ? 'tower' : 'single'),
    [recipe, setRecipe] = useState<ApartmentRecipeId | 'auto'>('gallery'),
    [detail, setDetail] = useState<'close' | 'district' | 'far'>(
      initialTower ? 'district' : 'close',
    );
  const [towerRecipe, setTowerRecipe] = useState<ResidentialTowerRecipe>(() => {
    const id = new URLSearchParams(location.search).get('tower');
    return id && id in RESIDENTIAL_TOWERS ? (id as ResidentialTowerRecipe) : 'ribbon';
  });
  const [stage, setStage] = useState<import('../types').Building['stage']>('open'),
    [progress, setProgress] = useState(0.85),
    [style, setStyle] = useState<'anime' | 'miniature'>('miniature');
  const [error, setError] = useState(''),
    [stats, setStats] = useState(''),
    [textures, setTextures] = useState('Loading textures');
  const params = useRef({
    width,
    depth,
    floors,
    seed,
    stage,
    progress,
    style,
    mode,
    recipe,
    detail,
    towerRecipe,
  });
  params.current = {
    width,
    depth,
    floors,
    seed,
    stage,
    progress,
    style,
    mode,
    recipe,
    detail,
    towerRecipe,
  };
  useEffect(() => {
    if (!mount.current) return;
    const host = mount.current,
      renderer = new THREE.WebGPURenderer({
        antialias: true,
        forceWebGL: new URLSearchParams(location.search).has('webgl'),
      });
    renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
    renderer.shadowMap.enabled = true;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    host.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#bbc8ca');
    const skyMaterial = new THREE.MeshBasicNodeMaterial({
      side: THREE.BackSide,
      depthWrite: false,
    });
    skyMaterial.colorNode = mix(
      color('#dde0d6'),
      color('#819eae'),
      smoothstep(-0.1, 0.85, positionLocal.normalize().y),
    );
    const sky = new THREE.Mesh(new THREE.SphereGeometry(1600, 24, 16), skyMaterial);
    scene.add(sky);
    const environment = new SkyEnvironment(sky);
    scene.environmentIntensity = 0.25;
    const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 2000);
    camera.position.set(31, 27, 40);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.target.set(0, 11, 0);
    controls.enableDamping = true;
    let pipeline: THREE.RenderPipeline | null = null,
      scenePass: ReturnType<typeof pass> | null = null,
      occlusion: ReturnType<typeof ao> | null = null;
    const resources = new ApartmentResources(),
      art = resources.art;
    let model: THREE.Group | null = null,
      disposed = false,
      ready = false;
    let currentView: 'building' | 'street' | 'rear' = 'building';
    const hemi = new THREE.HemisphereLight('#e0ebf3', '#7a817a', 1.35);
    scene.add(hemi);
    const sun = new THREE.DirectionalLight('#fff0d6', 3);
    sun.position.set(-25, 45, 30);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    Object.assign(sun.shadow.camera, { left: -30, right: 30, top: 45, bottom: -30, far: 450 });
    sun.shadow.normalBias = 0.04;
    scene.add(sun);
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(400, 400), art.get('#8b918b'));
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -0.05;
    ground.receiveShadow = true;
    scene.add(ground);
    const resize = () => {
      if (disposed) return;
      const w = host.clientWidth,
        h = host.clientHeight;
      camera.aspect = w / h;
      camera.setViewOffset(w, h, -Math.min(150, w * 0.18), 0, w, h);
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    const rebuildNow = () => {
      if (disposed) return;
      const p = params.current;
      art.setStyle(p.style);
      if (model) {
        scene.remove(model);
        disposeGroup(model);
      }
      const b: Building = {
        id: 1,
        parcel_id: 1,
        x: 0,
        z: 0,
        rotation: 0,
        width: p.width,
        depth: p.depth,
        floors: p.floors,
        appearance_seed: p.seed,
        kind: 'apartment',
        stage: p.stage,
        progress: p.stage === 'open' ? 1 : p.progress,
        capacity: 40,
        occupancy: 20,
        built_day: 0,
        name: 'Variable apartment pilot',
        history: [],
        anchor: false,
        vacant_days: 0,
        business_active: false,
      };
      const start = performance.now();
      model =
        p.mode === 'tower'
          ? buildResidentialTower(b, art, { recipe: p.towerRecipe, detail: p.detail })
          : p.mode === 'block'
            ? buildBlockStudy(art, p.seed, p.detail)
            : buildingKit(b, art, { apartmentComponents: { recipe: p.recipe, detail: p.detail } });
      if (!model) {
        setError('Tower requires width/depth ≥14m and 12–60 floors');
        return;
      }
      setError('');
      scene.add(model);
      let triangles = 0,
        meshes = 0;
      model.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          meshes++;
          triangles += (o.geometry.index?.count ?? o.geometry.getAttribute('position').count) / 3;
        }
      });
      ground.material = art.get('#8b918b');
      const direction = camera.position.clone().sub(controls.target).normalize();
      controls.target.set(0, Math.max(4, p.floors * 3.25 * 0.5 + 0.6), 0);
      camera.position
        .copy(controls.target)
        .addScaledVector(direction, Math.max(p.width, p.depth, p.floors * 3.25) * 2.5);
      if (p.mode === 'block') {
        controls.target.set(0, 9, 0);
        camera.position.set(180, 155, 210);
      }
      const reach =
        p.mode === 'block' ? 180 : p.mode === 'tower' ? Math.max(45, p.floors * 3.25 * 0.7) : 30;
      sun.position.copy(
        p.mode === 'tower'
          ? new THREE.Vector3(-reach, p.floors * 3.25 * 1.8, reach)
          : new THREE.Vector3(-25, 45, 30),
      );
      sun.target.position.set(0, p.mode === 'tower' ? p.floors * 3.25 * 0.4 : 0, 0);
      scene.add(sun.target);
      sun.shadow.bias = p.mode === 'tower' ? -0.00025 : 0;
      sun.shadow.normalBias = p.mode === 'tower' ? 0.13 : 0.04;
      sun.shadow.camera.far = Math.max(450, p.floors * 3.25 * 6);
      Object.assign(sun.shadow.camera, { left: -reach, right: reach, top: reach, bottom: -reach });
      sun.shadow.camera.updateProjectionMatrix();
      controls.update();
      view.current?.(currentView);
      const character = model.userData.apartmentPilot;
      const characterLabel = character
        ? ` · ${character.weathering?.profile ?? 'fresh'} · ${character.entrance?.placement ?? 'center'} entrance`
        : '';
      setStats(
        `${p.mode === 'block' ? '24 buildings · ' : ''}${meshes} material batches · ${Math.round(triangles).toLocaleString()} triangles · ${(performance.now() - start).toFixed(0)} ms build${characterLabel}`,
      );
    };
    view.current = (preset) => {
      currentView = preset;
      const p = params.current,
        front = (p.depth - 3.4) / 2 - 0.35;
      camera.fov = 38;
      camera.updateProjectionMatrix();
      if (p.mode === 'tower' && preset === 'street') {
        const height = p.floors * 3.25,
          distance = height * 0.95;
        camera.fov = 64;
        camera.updateProjectionMatrix();
        controls.target.set(0, height * 0.4, 0);
        camera.position.set(distance * 0.48, 5.5, distance * 0.88);
        controls.update();
        return;
      }
      if (p.mode === 'block') {
        controls.target.set(0, preset === 'street' ? 5 : 9, 0);
        camera.position.copy(
          preset === 'street'
            ? new THREE.Vector3(36, 9, 5)
            : preset === 'rear'
              ? new THREE.Vector3(-65, 65, -95)
              : new THREE.Vector3(180, 155, 210),
        );
        controls.update();
        return;
      }
      if (preset === 'street') {
        controls.target.set(0, 4.8, front);
        camera.position.set(p.width * 0.8, 6.4, front + 22);
      } else if (preset === 'rear') {
        controls.target.set(0, p.floors * 3.25 * 0.45, 0);
        camera.position.set(
          -p.width * 1.6,
          p.floors * 3.25 * 0.8,
          -Math.max(p.depth, p.floors * 3.25) * 2,
        );
      } else {
        controls.target.set(0, p.floors * 3.25 * 0.5 + 0.6, 0);
        camera.position
          .copy(controls.target)
          .add(
            new THREE.Vector3(31, 16, 40)
              .normalize()
              .multiplyScalar(Math.max(p.width, p.depth, p.floors * 3.25) * 2.2),
          );
      }
      controls.update();
    };
    rebuild.current = rebuildNow;
    rebuildNow();
    resources
      .loadTextures()
      .then((ok) => {
        if (disposed) return;
        rebuildNow();
        setTextures(
          ok
            ? '12 shared surface textures · material-specific finishes'
            : 'Texture load failed; plain fallback',
        );
      })
      .catch(() => {
        if (!disposed) setTextures('Texture load failed; plain fallback');
      });
    const observer = new ResizeObserver(resize);
    observer.observe(host);
    resize();
    const initialization = renderer.init();
    initialization
      .then(() => {
        if (disposed) return;
        ready = true;
        scene.environment = environment.update(renderer, 0, [new THREE.Color('#819eae')]);
        scenePass = pass(scene, camera, { samples: 4 });
        scenePass.setMRT(mrt({ output, normal: directionToColor(normalView) }));
        const normals = scenePass.getTextureNode('normal');
        occlusion = ao(
          scenePass.getTextureNode('depth'),
          sample((uv) => colorToDirection(normals.sample(uv))),
          camera,
        );
        occlusion.resolutionScale = 0.5;
        occlusion.radius.value = 0.85;
        occlusion.thickness.value = 0.8;
        occlusion.samples.value = 12;
        pipeline = new THREE.RenderPipeline(renderer);
        pipeline.outputNode = scenePass
          .getTextureNode('output')
          .mul(vec4(vec3(mix(float(1), occlusion.getTextureNode().r, float(0.65))), 1));
        renderer.setAnimationLoop(() => {
          controls.update();
          pipeline?.render();
        });
      })
      .catch((e) => {
        if (!disposed) setError(String(e));
      });
    return () => {
      disposed = true;
      rebuild.current = null;
      view.current = null;
      observer.disconnect();
      renderer.setAnimationLoop(null);
      controls.dispose();
      if (model) disposeGroup(model);
      ground.geometry.dispose();
      pipeline?.dispose();
      occlusion?.dispose();
      scenePass?.dispose();
      environment.dispose();
      sky.geometry.dispose();
      skyMaterial.dispose();
      resources.dispose();
      if (ready) renderer.dispose();
      else void initialization.then(() => renderer.dispose()).catch(() => {});
      renderer.domElement.remove();
    };
  }, []);
  useEffect(() => {
    rebuild.current?.();
  }, [width, depth, floors, seed, stage, progress, style, mode, recipe, detail, towerRecipe]);
  return (
    <main
      style={{
        position: 'fixed',
        inset: 0,
        fontFamily: 'system-ui',
        color: '#223b40',
        background: '#bbc8ca',
      }}
    >
      <div
        ref={mount}
        style={{ position: 'absolute', inset: 0 }}
        aria-label="Variable apartment 3D pilot"
      />
      <section
        style={{
          position: 'absolute',
          top: 16,
          left: 16,
          width: 270,
          padding: 18,
          background: '#f5f1e9ed',
          borderRadius: 6,
          maxHeight: '90vh',
          overflow: 'auto',
        }}
      >
        <h1 style={{ fontSize: 20, margin: '0 0 8px' }}>Architecture recipe study</h1>
        <p style={{ fontSize: 12 }}>
          Ten mid-rise and five tower recipes · seeded dimensions and detail levels. Drag to orbit;
          scroll to zoom.
        </p>
        <div style={{ display: 'flex', gap: 6 }}>
          {(['building', 'street', 'rear'] as const).map((preset) => (
            <button
              key={preset}
              onClick={() => view.current?.(preset)}
              style={{
                padding: '7px 10px',
                border: '1px solid #b6bcb6',
                background: '#fffaf0',
                borderRadius: 3,
                textTransform: 'capitalize',
              }}
            >
              {preset}
            </button>
          ))}
        </div>
        <label style={{ display: 'block', marginTop: 12 }}>
          Layout{' '}
          <select
            aria-label="Layout"
            value={mode}
            onChange={(e) => {
              const next = e.target.value as 'single' | 'block' | 'tower';
              setMode(next);
              if (next === 'block') setDetail('district');
              if (next === 'tower') {
                setWidth(26);
                setDepth(24);
                setFloors(32);
                setDetail('district');
              } else if (mode === 'tower') {
                setWidth(12);
                setDepth(14);
                setFloors(7);
              }
            }}
          >
            <option value="single">Single building</option>
            <option value="block">Mixed block · 24 buildings</option>
            <option value="tower">Residential tower options</option>
          </select>
        </label>
        <label style={{ display: 'block', marginTop: 10 }}>
          Recipe{' '}
          <select
            aria-label="Recipe"
            value={mode === 'tower' ? towerRecipe : recipe}
            disabled={mode === 'block'}
            onChange={(e) =>
              mode === 'tower'
                ? setTowerRecipe(e.target.value as ResidentialTowerRecipe)
                : setRecipe(e.target.value as typeof recipe)
            }
          >
            {mode === 'tower' ? (
              Object.entries(RESIDENTIAL_TOWERS).map(([id, r]) => (
                <option key={id} value={id}>
                  {r.label}
                </option>
              ))
            ) : (
              <>
                <option value="auto">Seeded selection</option>
                {APARTMENT_RECIPE_IDS.map((id) => (
                  <option key={id} value={id}>
                    {APARTMENT_RECIPES[id].label}
                  </option>
                ))}
              </>
            )}
          </select>
        </label>
        <label style={{ display: 'block', marginTop: 10 }}>
          Detail{' '}
          <select
            aria-label="Detail"
            value={detail}
            onChange={(e) => setDetail(e.target.value as typeof detail)}
          >
            <option value="close">Close inspection</option>
            <option value="district">District</option>
            <option value="far">Distant</option>
          </select>
        </label>
        {(
          [
            ['Width', width, mode === 'tower' ? 14 : 5, mode === 'tower' ? 40 : 24, 0.5, setWidth],
            ['Depth', depth, mode === 'tower' ? 14 : 8, mode === 'tower' ? 40 : 24, 0.5, setDepth],
            ['Floors', floors, mode === 'tower' ? 12 : 4, mode === 'tower' ? 60 : 11, 1, setFloors],
            ['Seed', seed, 0, 99, 1, setSeed],
          ] as const
        ).map(([label, value, min, max, step, set]) => (
          <label key={label} style={{ display: 'block', margin: '14px 0', fontSize: 13 }}>
            {label}: {value}
            {label === 'Width' || label === 'Depth' ? ' m' : ''}
            <input
              disabled={mode === 'block' && label !== 'Seed'}
              aria-label={label}
              type="range"
              min={min}
              max={max}
              step={step}
              value={value}
              onChange={(e) => set(Number(e.target.value))}
              style={{ display: 'block', width: '100%' }}
            />
          </label>
        ))}
        <label>
          Stage{' '}
          <select
            disabled={mode === 'block'}
            aria-label="Stage"
            value={stage}
            onChange={(e) => setStage(e.target.value as import('../types').Building['stage'])}
          >
            {['open', 'foundation', 'structure', 'surface', 'finishing', 'renovation'].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
        {stage !== 'open' && (
          <label style={{ display: 'block', marginTop: 14 }}>
            Progress: {Math.round(progress * 100)}%
            <input
              aria-label="Progress"
              type="range"
              min="0.4"
              max="1"
              step=".01"
              value={progress}
              onChange={(e) => setProgress(Number(e.target.value))}
            />
          </label>
        )}
        <label style={{ display: 'block', marginTop: 14 }}>
          Style{' '}
          <select
            aria-label="Style"
            value={style}
            onChange={(e) => setStyle(e.target.value as typeof style)}
          >
            <option value="miniature">Miniature</option>
            <option value="anime">Anime</option>
          </select>
        </label>
        <p style={{ fontSize: 11 }}>{textures}</p>
        <output style={{ fontSize: 11 }}>{stats}</output>
        {error && <p role="alert">{error}</p>}
        <p style={{ fontSize: 11 }}>
          {mode === 'tower'
            ? 'Five procedural tower recipes · podium, recessed glazing, chamfered balconies and service crown.'
            : mode === 'block'
              ? 'Synthetic mixed block: ten apartment recipes plus existing houses and shops.'
              : 'City-integrated apartment family. Foundation, frame and cladding share the same envelope.'}
        </p>
      </section>
    </main>
  );
}
