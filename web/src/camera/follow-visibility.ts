import { cameraPathClearance, type CameraObstacleBox, type CameraPoint3 } from './clearance';

/** A building is stationary and volumetric, unlike a train's linear consist.
 * Exclude its own envelope from sight tests; its facade is the intended subject. */
export function chooseBuildingFollowPosition(
  target: CameraPoint3, radius: number, distance: number, obstacles: readonly CameraObstacleBox[],
): {position: CameraPoint3; visible: boolean} {
  const nearby = obstacles.filter(box =>
    Math.hypot(box.center.x-target.x, box.center.z-target.z) > .01 &&
    Math.hypot(box.center.x-target.x, box.center.z-target.z) <= distance + Math.hypot(box.width,box.depth)/2);
  // Check the foot of the site as well as its upper mass. Looking only at the
  // former roof can report a clear shot while a demolished low site is hidden.
  const subjects = [target, { ...target, y: Math.max(1, target.y - radius * .6) }, { ...target, y: target.y + radius * .6 }];
  let best: CameraPoint3 = target, score = -1;
  for (const slope of [.9, 1.4, 2.4, 4]) {
    for (const yaw of [.75, .75+Math.PI/2, .75-Math.PI/2, .75+Math.PI]) {
      const horizontal = distance / Math.sqrt(1+slope*slope);
      const position = { x: Math.max(-430, Math.min(430,target.x+Math.sin(yaw)*horizontal)),
        y: Math.min(700,target.y+slope*horizontal),
        z: Math.max(-430,Math.min(430,target.z+Math.cos(yaw)*horizontal)) };
      const visible = subjects.filter(subject => !cameraPathClearance(subject,position,nearby,{terrainY:0,safetyMarginM:.3}).collides).length;
      if (visible > score) { best = position; score = visible; }
      if (visible === subjects.length) return { position, visible: true };
    }
  }
  return { position: best, visible: false };
}

/** Bounded shot selection, independent of simulation and camera travel planning. */
export function chooseTrainFollowPosition(
  target: CameraPoint3, forward: {x:number;z:number}, preferred: CameraPoint3,
  distance: number, obstacles: readonly CameraObstacleBox[],
): {position:CameraPoint3;visible:boolean} {
  const length = Math.hypot(forward.x, forward.z) || 1;
  const subjects = [-20, 0, 20].map(offset => ({x:target.x+forward.x/length*offset,y:target.y,z:target.z+forward.z/length*offset}));
  // Far buildings cannot intersect this bounded shot; filter once for all candidates.
  const nearby = obstacles.filter(box => Math.hypot(box.center.x-target.x,box.center.z-target.z)
    <= distance + Math.hypot(box.width,box.depth)/2 + 22);
  const yaw = Math.atan2(preferred.x, preferred.z);
  const baseSlope = preferred.y / Math.max(.001, Math.hypot(preferred.x,preferred.z));
  let fallback:CameraPoint3|undefined, bestVisible = -1;
  for (const slope of [baseSlope, Math.max(.65,baseSlope), Math.max(1.2,baseSlope), Math.max(2.4,baseSlope)]) {
    for (const turn of [0, Math.PI/6, -Math.PI/6, Math.PI/3, -Math.PI/3, Math.PI]) {
      const horizontal = distance / Math.sqrt(1+slope*slope);
      const position = {x:Math.max(-430,Math.min(430,target.x+Math.sin(yaw+turn)*horizontal)),
        y:Math.min(700,target.y+slope*horizontal),z:Math.max(-430,Math.min(430,target.z+Math.cos(yaw+turn)*horizontal))};
      const visible = subjects.filter(subject => !cameraPathClearance(subject,position,nearby,{terrainY:0,safetyMarginM:.3}).collides).length;
      if (visible > bestVisible) {fallback=position;bestVisible=visible;}
      if (visible === subjects.length) return {position,visible:true};
    }
  }
  return {position:fallback!,visible:false};
}

/** Favor a readable entrance elevation over a clear view of only the roof. */
export function chooseFrontageWatchPosition(
  target: CameraPoint3, width: number, depth: number, height: number, rotation: number,
  distance: number, obstacles: readonly CameraObstacleBox[],
): { position: CameraPoint3; visible: boolean } {
  const sin = Math.sin(rotation), cos = Math.cos(rotation);
  const neighbors = obstacles.filter(box => Math.hypot(box.center.x-target.x, box.center.z-target.z) > .01
    && Math.hypot(box.center.x-target.x, box.center.z-target.z) <= distance + Math.hypot(box.width, box.depth)/2);
  const subjects = [-.3, 0, .3].map(side => ({
    x: target.x + cos * width * side + sin * depth / 2,
    y: Math.min(2, height / 2),
    z: target.z - sin * width * side + cos * depth / 2,
  }));
  for (const slope of [.65, .9, 1.4, 2.4]) {
    for (const turn of [0, Math.PI/6, -Math.PI/6, Math.PI/3, -Math.PI/3]) {
      const yaw = rotation + turn, horizontal = distance / Math.sqrt(1+slope*slope);
      const position = { x: Math.max(-430, Math.min(430, target.x+Math.sin(yaw)*horizontal)),
        y: Math.min(700, target.y+slope*horizontal),
        z: Math.max(-430, Math.min(430, target.z+Math.cos(yaw)*horizontal)) };
      if (subjects.every(subject => !cameraPathClearance(subject, position, neighbors,
        { terrainY: 0, safetyMarginM: .3 }).collides)) return { position, visible: true };
    }
  }
  return chooseBuildingFollowPosition(target, Math.hypot(width/2, depth/2, height/2), distance, obstacles);
}
