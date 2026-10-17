// Composition: derives the audio director's scene input from what the renderer presents.
import * as THREE from 'three/webgpu';
import { resolveAudioEventContext } from '../audio/event-context';
import { distanceToSegment, type PresentationView } from '../render/city';
import { isRoadVehicle } from '../render/vehicles';

export function audioSceneContext(view: PresentationView) {
  const { world, listener } = view;
  const v = view.camera.getWorldDirection(new THREE.Vector3());
  const trains: Array<{ id: number; x: number; z: number; speed: number }> = [],
    buses: Array<{ id: number; x: number; z: number; speed: number }> = [],
    traffic: Array<{ id: number; x: number; z: number; speed: number; distance: number }> = [];
  let pedestrians = 0;
  for (const [id, p] of view.current) {
    const old = view.previous.get(id) ?? p,
      speed = Math.hypot(p.x - old.x, p.z - old.z) / view.motionDelta,
      distance = Math.hypot(p.x - listener.x, p.z - listener.z);
    if (p.k === 2) trains.push({ id, x: p.x, z: p.z, speed });
    else if (p.k === 3) buses.push({ id, x: p.x, z: p.z, speed });
    else if (isRoadVehicle(p.k) && distance < 130)
      traffic.push({ id, x: p.x, z: p.z, speed, distance });
    else if (p.k === 0 && distance < 45) pedestrians++;
  }
  traffic.sort((a, b) => a.distance - b.distance || a.id - b.id);
  const events = resolveAudioEventContext(world, view.current, view.motionTick);
  let riverDistance = 800;
  const river = world?.terrain.river ?? [];
  for (let i = 0; i < river.length - 1; i++)
    riverDistance = Math.min(
      riverDistance,
      Math.max(
        0,
        distanceToSegment({ x: listener.x, z: listener.z }, river[i], river[i + 1]) -
          view.riverHalfWidth,
      ),
    );
  // Listen at the observed scene, with compressed height appropriate to a
  // miniature world, rather than silencing it from a distant aerial camera.
  const construction = [
    ...(world?.rear_extensions ?? [])
      .slice(0, 2)
      .map((e) => ({ id: e.project_id, x: e.x, z: e.z })),
    ...(world?.buildings
      .filter((b) => b.stage !== 'open')
      .map((b) => ({ id: b.id, x: b.x, z: b.z })) ?? []),
  ].slice(0, 12);
  return {
    hour: view.hour,
    rain: view.rain,
    riverDistance,
    tick: view.motionTick,
    worldKey: world?.seed,
    developmentRate: world?.development_rate,
    traffic: traffic.slice(0, 24),
    pedestrians,
    events,
    camera: {
      x: listener.x,
      y: view.camera.position.y * 0.15,
      z: listener.z,
      forward: { x: v.x, z: v.z },
    },
    trains,
    buses,
    construction,
  };
}
