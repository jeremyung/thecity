import { MOVEMENT_STRIDE } from '../movement';
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { LENS_PRESETS, verticalFovDegrees, type LensPresetId } from "./lens";
import { cameraPathClearance, type CameraObstacleBox, type CameraPoint3 } from "./clearance";
import { planAutomaticCameraPath } from "./automatic-path";
import type { BusLineView, BusView, Point, Project, World } from "../types";

export type CameraMode = "watch" | "explore" | "overview";
type Anchor = {
  id: string;
  label: string;
  x: number;
  z: number;
  targetY?: number;
  /** World-space direction from the framed subject toward the camera. */
  cameraDirection?: THREE.Vector3;
  frameRadius?: number;
  /** Actual rotated facade/roof corners for perspective-safe skyline framing. */
  framePoints?: THREE.Vector3[];
  importance: number;
  activity: number;
  suitability: "all" | "night" | "rain";
};
type Extent = { x: number; z: number; span: number };
type SkylineTransition = { elapsed: number; duration: number; startPosition: THREE.Vector3; startTarget: THREE.Vector3; controlPosition: THREE.Vector3 };
type AutomaticRoute = {
  waypoints: THREE.Vector3[];
  index: number;
  requestedPosition: THREE.Vector3;
  requestedTarget: THREE.Vector3;
  obstacleRevision: number;
  lifted: boolean;
};
type SettledAutomaticPose = {
  position: THREE.Vector3;
  target: THREE.Vector3;
  obstacleRevision: number;
};
type FollowResident = {
  id: number;
  name: string;
  home_building: number;
  work_building: number | null;
  activity: string;
  transfer?: string;
  train_id?: number | null;
  bus_id?: number | null;
  bus_origin_stop?: number | null;
  bus_destination_stop?: number | null;
  bus_stop_node?: number | null;
  location_node?: number | null;
};
type FollowTrain = World["trains"][number] & { direction?: number };
type FollowWorld = World & { residents?: FollowResident[]; trains: FollowTrain[]; buses?: BusView[]; bus_lines?: BusLineView[] };

const MAX_ANCHORS = 32;
const WORLD_LIMIT = 430;
const MIN_HEIGHT = 12;
const UP = new THREE.Vector3(0, 1, 0);
// Keep enough sky and waterfront in frame to read the city as a place, while
// retaining the higher angle needed to understand its street fabric.
const WATCH_DIRECTION = new THREE.Vector3(0.65, 0.28, 0.7).normalize();
const OVERVIEW_DIRECTION = new THREE.Vector3(0.65, 0.42, 0.7).normalize();
const KEY_TARGETS = "input, textarea, select, button, [contenteditable='true'], [role='textbox']";

/** Smooth scene direction around one shared world camera. */
export class CameraDirector {
  readonly controls: OrbitControls;
  onModeChange?: (mode: CameraMode) => void;

  private currentMode: CameraMode = "watch";
  private currentWorld: World | null = null;
  /** Conservative full-parcel obstruction envelopes, rebuilt on each snapshot. */
  private cameraObstacles: CameraObstacleBox[] = [];
  private obstacleSignature = "";
  private obstacleRevision = 0;
  private automaticRoute: AutomaticRoute | null = null;
  private settledAutomaticPose: SettledAutomaticPose | null = null;
  private anchors: Anchor[] = [];
  private lastVisit = new Map<string, number>();
  private selectedAnchor: Anchor | null = null;
  private skylineTransition: SkylineTransition | null = null;
  private desiredTarget = new THREE.Vector3(50, 0, 20);
  private desiredPosition = new THREE.Vector3(240, 170, 290);
  private followId: number | null = null;
  private followPositionValid = false;
  private followMovementPresent = false;
  private followLostSeconds = 0;
  private readonly followPosition = new THREE.Vector3();
  private readonly lastClearManualPosition = new THREE.Vector3();
  private readonly lastClearManualTarget = new THREE.Vector3();
  private readonly followForward = new THREE.Vector3();
  private readonly followViewDirection=WATCH_DIRECTION.clone();
  private followHeading = 0;
  private followKind = 0;
  private elapsed = 0;
  private holdSeconds = 0;
  private holdDuration = 52;
  private blendSeconds = 8;
  private growthAge = 0;
  private lastRevealAt = -Infinity;
  private baselineFootprint = 0;
  private overviewInitialized = false;
  private framingAspect = 0;
  private extent: Extent = { x: 50, z: 20, span: 120 };
  private revealSeconds = 0;
  private reducedMotion = false;
  private mediaQuery: MediaQueryList | null = null;
  private disposed = false;
  private currentLens: LensPresetId = "environment";

  private readonly onControlsStart = () => this.takeManualControl();
  private readonly onKeyDown = (event: KeyboardEvent) => this.handleKeyDown(event);
  private readonly onReducedMotionChange = (event: MediaQueryListEvent) => { this.reducedMotion = event.matches; };

  constructor(readonly camera: THREE.PerspectiveCamera, readonly element: HTMLElement) {
    camera.position.set(240, 170, 290);
    this.controls = new OrbitControls(camera, element);
    this.controls.target.set(50, 0, 20);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.075;
    this.controls.screenSpacePanning = false;
    this.controls.minDistance = 45;
    this.controls.maxDistance = 700;
    this.controls.minPolarAngle = 0.14;
    this.controls.maxPolarAngle = 1.32;
    this.controls.addEventListener("start", this.onControlsStart);
    this.element.addEventListener("keydown", this.onKeyDown);
    this.element.tabIndex = this.element.tabIndex < 0 ? 0 : this.element.tabIndex;
    this.mediaQuery = typeof window !== "undefined" && window.matchMedia
      ? window.matchMedia("(prefers-reduced-motion: reduce)")
      : null;
    this.reducedMotion = this.mediaQuery?.matches ?? false;
    this.mediaQuery?.addEventListener?.("change", this.onReducedMotionChange);
    this.mediaQuery?.addListener?.(this.onReducedMotionChange);
    this.controls.update();
    this.framingAspect = this.validAspect();
    this.desiredPosition.copy(camera.position);
    this.desiredTarget.copy(this.controls.target);
    this.lastClearManualPosition.copy(camera.position);
    this.lastClearManualTarget.copy(this.controls.target);
  }

  get mode(): CameraMode { return this.currentMode; }

  get lens(): LensPresetId { return this.currentLens; }

  /** Change focal framing without moving the camera; user changes take Explore control. */
  setLens(id: LensPresetId, takeManualControl = true): void {
    if (this.disposed || this.currentLens === id) return;
    const preset = LENS_PRESETS[id];
    if (!preset) throw new RangeError(`Unknown camera lens preset: ${String(id)}`);
    if (takeManualControl) this.takeManualControl();
    this.currentLens = id;
    this.camera.fov = verticalFovDegrees(preset.focalLengthMm);
    this.camera.updateProjectionMatrix();
  }

  get label(): string {
    if (this.followId !== null) {
      const entity = this.findEntity(this.followId);
      if (entity) return entity;
      return "Following subject";
    }
    if (this.currentMode === "overview") return "City overview";
    if (this.currentMode === "explore") return "Explore";
    return this.selectedAnchor?.label ?? "City watch";
  }

  /** The current OrbitControls focus point. */
  get target(): THREE.Vector3 { return this.controls.target; }

  world(world: World): void {
    if (this.disposed) return;
    const firstWorld = this.currentWorld === null;
    const replacedWorld = this.currentWorld !== null
      && (world.seed !== this.currentWorld.seed || world.tick < this.currentWorld.tick);
    const initializeWatchPose = firstWorld || replacedWorld;
    if (initializeWatchPose) {
      this.selectedAnchor = null;
      this.lastVisit.clear();
      this.growthAge = 0;
      this.lastRevealAt = -Infinity;
      this.baselineFootprint = 0;
      this.revealSeconds = 0;
      this.overviewInitialized = false;
    }
    this.currentWorld = world;
    const obstacles = this.cameraObstacleEnvelopes(world);
    const obstacleSignature = JSON.stringify(obstacles);
    const obstacleRevisionChanged = initializeWatchPose || obstacleSignature !== this.obstacleSignature;
    if (obstacleRevisionChanged) {
      this.obstacleSignature = obstacleSignature;
      this.obstacleRevision++;
      this.automaticRoute = null;
      this.settledAutomaticPose = null;
    }
    this.cameraObstacles = obstacles;
    this.anchors = this.buildAnchors(world);
    for (const id of this.lastVisit.keys()) {
      if (!this.anchors.some((anchor) => anchor.id === id)) this.lastVisit.delete(id);
    }
    this.extent = this.developedExtent(world);
    if (this.baselineFootprint === 0) this.baselineFootprint = this.extent.span;

    // A manual view is preserved across snapshots unless a new structure now
    // intersects the camera. Initial Watch framing is selected below first so
    // recovery is based on the actual opening shot, not the constructor pose.
    if (this.currentMode === "explore" && this.currentPoseNeedsRecovery()) {
      this.recoverAndRememberManualPose();
    }

    // A newly added or taller structure can enclose an otherwise settled
    // Watch/Follow pose. Recover at snapshot time, before updateAutomaticControls
    // performs its swept-motion guard; otherwise that guard would correctly
    // reject motion from the now-invalid start forever. The first/replaced
    // Watch world still frames its anchor before the existing initialization
    // recovery below.
    if (obstacleRevisionChanged && !initializeWatchPose && this.currentMode !== "explore") {
      this.recoverAutomaticPoseFromSnapshot();
    }

    if (this.followId !== null) {
      // Movement tuples are the live position. Structural snapshots provide an
      // indoor or onboard fallback only when the latest tuple omitted the subject.
      if (!this.followMovementPresent) {
        this.followPositionValid = this.findEntityPoint(this.followId, this.followPosition);
        if (this.followPositionValid) this.followLostSeconds = 0;
      }
      return;
    }

    if (this.currentMode === "watch" && this.selectedAnchor && !this.anchors.some((anchor) => anchor.id === this.selectedAnchor?.id)) {
      this.selectNextAnchor();
    }
    if (this.currentMode === "watch" && !this.selectedAnchor) this.selectNextAnchor(initializeWatchPose);
    if (this.currentMode === "overview" && !this.overviewInitialized) {
      this.frameOverview(14);
      this.overviewInitialized = true;
    }
    // Initial/replaced snapshots may move the camera above a newly occupied
    // location. This is a one-time view initialization, never a per-frame lift.
    if (initializeWatchPose && this.currentMode !== "explore") this.recoverAndRememberManualPose();
  }

  update(dt: number, _time: number): void {
    if (this.disposed) return;
    const step = Math.max(0, Math.min(dt, 0.1));
    this.elapsed += step;
    if (this.currentMode !== "explore" || this.followId !== null) {
      if (!this.updateAutomaticControls()) return;
    } else this.controls.update();
    this.refreshFramingForAspect();
    if (this.currentMode === "explore" && this.followId === null) {
      this.clampControls();
      this.validateManualMotion();
      return;
    }

    if (this.followId !== null) {
      this.updateFollow(step);
      this.applyPose(step, 0.38);
      return;
    }

    if (this.skylineTransition) {
      this.applyPose(step, this.blendSeconds);
      return;
    }

    if (!this.reducedMotion) this.updateFootprintReveal(step);
    if (this.currentMode === "overview") {
      if (this.revealSeconds > 0) this.revealSeconds = Math.max(0, this.revealSeconds - step);
      this.applyPose(step, this.blendSeconds);
      return;
    }

    if (this.revealSeconds > 0) {
      this.revealSeconds = Math.max(0, this.revealSeconds - step);
      this.applyPose(step, this.blendSeconds);
      if (this.revealSeconds === 0) this.restoreSelectedAnchor();
      return;
    }

    if (this.reducedMotion) {
      this.applyPose(step, 0.5);
      return;
    }

    this.holdSeconds += step;
    if (this.holdSeconds >= this.holdDuration && this.anchors.length > 1) this.selectNextAnchor();
    this.applyPose(step, this.blendSeconds);
  }

  setMode(mode: CameraMode): void {
    if (this.disposed || this.currentMode === mode && !(mode === "watch" && this.followId !== null)) return;
    this.currentMode = mode;
    this.skylineTransition = null;
    this.automaticRoute = null;
    this.settledAutomaticPose = null;
    this.followId = null;
    this.followPositionValid = false;
    this.followMovementPresent = false;
    this.revealSeconds = 0;
    if (mode === "watch") {
      this.selectNextAnchor();
    } else if (mode === "overview") {
      this.frameOverview(16);
      this.overviewInitialized = true;
    } else {
      this.desiredPosition.copy(this.camera.position);
      this.desiredTarget.copy(this.controls.target);
      this.recoverAndRememberManualPose();
    }
    this.onModeChange?.(mode);
  }

  follow(id: number | null): void {
    if (this.disposed) return;
    this.skylineTransition = null;
    this.automaticRoute = null;
    this.settledAutomaticPose = null;
    this.followId = id;
    this.followLostSeconds = 0;
    this.followPositionValid = false;
    this.followMovementPresent = false;
    if (id === null) {
      this.desiredPosition.copy(this.camera.position);
      this.desiredTarget.copy(this.controls.target);
      this.holdSeconds = 0;
      return;
    }
    this.followPositionValid = this.findEntityPoint(id, this.followPosition);
    if (this.followPositionValid) this.updateFollow(0);
  }

  /** Explicit developer review action; this is a normal eased Watch scene selection. */
  reviewSkyline(): boolean {
    if (this.disposed) return false;
    const anchor = this.anchors.find((candidate) => candidate.id.startsWith("skyline:"));
    if (!anchor) return false;
    this.currentMode = "watch";
    this.followId = null;
    this.automaticRoute = null;
    this.settledAutomaticPose = null;
    this.followPositionValid = false;
    this.followMovementPresent = false;
    this.selectedAnchor = anchor;
    this.lastVisit.set(anchor.id, this.elapsed);
    this.holdSeconds = 0;
    this.holdDuration = 45 + this.hash(anchor.id) % 46;
    this.revealSeconds = 0;
    this.blendSeconds = this.reducedMotion ? 0.5 : 5.5;
    this.setAnchorPose(anchor, this.anchorDistance(anchor));
    this.beginSkylineTransition(anchor);
    this.onModeChange?.("watch");
    return true;
  }

  /** Copy only the selected subject's tuple [id,x,z,heading,kind,surfaceY]. */
  setPositions(data: Float32Array): void {
    if (this.followId === null || this.disposed) return;
    this.followMovementPresent = false;
    const resident = this.findResident(this.followId);
    const movementId = resident?.bus_id ?? resident?.train_id ?? this.followId;
    for (let index = 0; index + MOVEMENT_STRIDE <= data.length; index += MOVEMENT_STRIDE) {
      if (data[index] !== movementId) continue;
      this.followPosition.set(data[index + 1], Math.max(0,data[index + 5]), data[index + 2]);
      this.followHeading = data[index + 3];
      this.followKind = data[index + 4];
      this.followPositionValid = true;
      this.followMovementPresent = true;
      this.followLostSeconds = 0;
      return;
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.controls.removeEventListener("start", this.onControlsStart);
    this.controls.dispose();
    this.element.removeEventListener("keydown", this.onKeyDown);
    this.mediaQuery?.removeEventListener?.("change", this.onReducedMotionChange);
    this.mediaQuery?.removeListener?.(this.onReducedMotionChange);
    this.anchors = [];
    this.lastVisit.clear();
  }

  private takeManualControl(): void {
    if (this.currentMode === "explore" && this.followId === null) return;
    this.currentMode = "explore";
    this.skylineTransition = null;
    this.automaticRoute = null;
    this.settledAutomaticPose = null;
    this.followId = null;
    this.followPositionValid = false;
    this.followMovementPresent = false;
    this.revealSeconds = 0;
    this.recoverAndRememberManualPose();
    this.desiredPosition.copy(this.camera.position);
    this.desiredTarget.copy(this.controls.target);
    this.onModeChange?.("explore");
  }

  private cameraObstacleEnvelopes(world: World): CameraObstacleBox[] {
    return world.buildings.map((building) => {
      const tower = building.floors >= 12 && (building.kind === "office" || building.kind === "apartment");
      const floors = Math.max(1, building.floors);
      const base = 0.58;
      const roofAllowance = building.stage !== "open" ? 5 : tower ? 4.05 : 2.75;
      // Use full authoritative parcel dimensions, then include facade, canopy,
      // and construction scaffold allowances. The envelope is intentionally
      // conservative; it is a camera obstacle, not a rendered footprint.
      return {
        center: { x: building.x, z: building.z },
        rotation: building.rotation,
        width: building.width + 2.4,
        depth: building.depth + 5.2,
        minY: 0,
        maxY: base + floors * 3.25 + roofAllowance,
      };
    });
  }

  private cameraPoint(position: THREE.Vector3): CameraPoint3 {
    return { x: position.x, y: position.y, z: position.z };
  }

  private currentPoseNeedsRecovery(): boolean {
    const point = this.cameraPoint(this.camera.position);
    const clearance = cameraPathClearance(point, point, this.cameraObstacles, { terrainY: 0, safetyMarginM: 1.5 });
    return clearance.collides || this.camera.position.y < clearance.requiredCameraHeight;
  }

  /** Snapshot-time exception: raise an already-obstructed pose once, preserving XZ and target. */
  private recoverAndRememberManualPose(): void {
    this.recoverPoseFromCurrentObstacles();
    this.lastClearManualPosition.copy(this.camera.position);
    this.lastClearManualTarget.copy(this.controls.target);
  }

  /** One-time snapshot recovery; later camera travel remains segment-guarded. */
  private recoverAutomaticPoseFromSnapshot(): void {
    this.recoverPoseFromCurrentObstacles();
  }

  private recoverPoseFromCurrentObstacles(): void {
    const position = this.camera.position;
    const point = this.cameraPoint(position);
    const clearance = cameraPathClearance(point, point, this.cameraObstacles, { terrainY: 0, safetyMarginM: 1.5 });
    if (clearance.collides || position.y < clearance.requiredCameraHeight) {
      // Tangency is treated as collision by the sweep test, so leave a small
      // explicit gap above the required clearance plane.
      position.y = clearance.requiredCameraHeight + 0.25;
      // OrbitControls derives its spherical baseline from the corrected pose.
      // Relax its normal framing bounds for this single synchronization so a
      // high recovery pose is not immediately clamped back into the structure.
      const { minDistance, maxDistance, minPolarAngle, maxPolarAngle, enableDamping } = this.controls;
      this.controls.minDistance = 0;
      this.controls.maxDistance = 1_200;
      this.controls.minPolarAngle = 0;
      this.controls.maxPolarAngle = Math.PI;
      this.controls.enableDamping = false;
      this.controls.update();
      this.controls.minDistance = minDistance;
      this.controls.maxDistance = maxDistance;
      this.controls.minPolarAngle = minPolarAngle;
      this.controls.maxPolarAngle = maxPolarAngle;
      this.controls.enableDamping = enableDamping;
      this.camera.lookAt(this.controls.target);
      this.camera.updateMatrixWorld(true);

      const synchronized = this.cameraPoint(position);
      if (cameraPathClearance(synchronized, synchronized, this.cameraObstacles, { terrainY: 0, safetyMarginM: 1.5 }).collides) {
        // Preserve the explicit vertical recovery even if a controls adapter
        // applies an unexpected clamp. Rebuild orientation to match its target.
        position.set(point.x, clearance.requiredCameraHeight + 0.25, point.z);
        this.camera.lookAt(this.controls.target);
        this.camera.updateMatrixWorld(true);
      }
    }
  }

  private validateManualMotion(): void {
    const clearance = cameraPathClearance(
      this.cameraPoint(this.lastClearManualPosition),
      this.cameraPoint(this.camera.position),
      this.cameraObstacles,
      { terrainY: 0, safetyMarginM: 1.5 },
    );
    if (clearance.collides) {
      // Consume OrbitControls' residual damping before restoring the last clear
      // pose; otherwise its next update would keep pushing into the obstacle.
      const damping = this.controls.enableDamping;
      this.controls.enableDamping = false;
      this.controls.update();
      this.controls.enableDamping = damping;
      this.camera.position.copy(this.lastClearManualPosition);
      this.controls.target.copy(this.lastClearManualTarget);
      this.controls.update();
      this.desiredPosition.copy(this.lastClearManualPosition);
      this.desiredTarget.copy(this.lastClearManualTarget);
      return;
    }
    this.lastClearManualPosition.copy(this.camera.position);
    this.lastClearManualTarget.copy(this.controls.target);
  }

  private handleKeyDown(event: KeyboardEvent): void {
    if (this.disposed || event.target instanceof Element && event.target.matches(KEY_TARGETS)) return;
    const key = event.key;
    if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "+", "=", "-", "_"].includes(key)) return;
    event.preventDefault();
    this.takeManualControl();

    if (key === "+" || key === "=" || key === "-" || key === "_") {
      const offset = new THREE.Vector3().subVectors(this.camera.position, this.controls.target);
      const distance = THREE.MathUtils.clamp(offset.length() * (key === "+" || key === "=" ? 0.88 : 1.14), 45, 700);
      offset.setLength(distance);
      this.camera.position.copy(this.controls.target).add(offset);
    } else {
      const right = new THREE.Vector3(1, 0, 0).applyQuaternion(this.camera.quaternion);
      right.y = 0;
      right.normalize();
      const forward = new THREE.Vector3().crossVectors(UP, right).normalize();
      const direction = key === "ArrowLeft" ? right.negate()
        : key === "ArrowRight" ? right
          : key === "ArrowUp" ? forward
            : forward.negate();
      const distance = Math.max(6, this.camera.position.distanceTo(this.controls.target) * 0.08);
      direction.multiplyScalar(distance);
      this.controls.target.add(direction);
      this.camera.position.add(direction);
    }
    this.clampControls();
    this.controls.update();
  }

  private clampControls(): void {
    this.controls.target.x = THREE.MathUtils.clamp(this.controls.target.x, -WORLD_LIMIT, WORLD_LIMIT);
    this.controls.target.z = THREE.MathUtils.clamp(this.controls.target.z, -WORLD_LIMIT, WORLD_LIMIT);
    const offset = new THREE.Vector3().subVectors(this.camera.position, this.controls.target);
    const distance = THREE.MathUtils.clamp(offset.length(), this.controls.minDistance, this.controls.maxDistance);
    offset.setLength(distance);
    this.camera.position.copy(this.controls.target).add(offset);
    if (this.camera.position.y < this.controls.target.y + MIN_HEIGHT) {
      this.camera.position.y = this.controls.target.y + MIN_HEIGHT;
    }
    this.controls.update();
  }

  private applyPose(dt: number, seconds: number): void {
    if (this.skylineTransition && !this.reducedMotion) {
      const transition = this.skylineTransition;
      const previousPosition = this.camera.position.clone();
      const previousTarget = this.controls.target.clone();
      const previousQuaternion = this.camera.quaternion.clone();
      transition.elapsed += Math.max(0, dt);
      const raw = THREE.MathUtils.clamp(transition.elapsed / transition.duration, 0, 1);
      const t = raw * raw * (3 - 2 * raw);
      const inverse = 1 - t;
      const nextPosition = transition.startPosition.clone().multiplyScalar(inverse * inverse)
        .addScaledVector(transition.controlPosition, 2 * inverse * t)
        .addScaledVector(this.desiredPosition, t * t);
      if (cameraPathClearance(
        this.cameraPoint(this.camera.position), this.cameraPoint(nextPosition), this.cameraObstacles,
        { terrainY: 0, safetyMarginM: 1.5 },
      ).collides) {
        // A skyline arc is an authored convenience, but it still yields to the
        // same swept-obstacle planner as every other automatic transition.
        this.skylineTransition = null;
        this.automaticRoute = null;
        this.applyAutomaticPose(dt, seconds);
        return;
      }
      this.camera.position.copy(nextPosition);
      const nextTarget = transition.startTarget.clone().lerp(this.desiredTarget, t);
      this.controls.target.copy(nextTarget);
      if (raw >= 1) {
        this.camera.position.copy(this.desiredPosition);
        this.controls.target.copy(this.desiredTarget);
        this.skylineTransition = null;
      }
      this.updateAutomaticControls(previousPosition, previousTarget, previousQuaternion);
      return;
    }
    if (this.reducedMotion) this.skylineTransition = null;
    this.applyAutomaticPose(dt, seconds);
  }

  /** Ease along planner-verified segments; never interpolate across a corner. */
  private applyAutomaticPose(dt: number, seconds: number): void {
    const settled = this.settledAutomaticPose;
    if (!this.automaticRoute && settled && settled.obstacleRevision === this.obstacleRevision
      && settled.position.distanceTo(this.desiredPosition) < 0.05
      && settled.target.distanceTo(this.desiredTarget) < 0.05
      && this.camera.position.distanceTo(this.desiredPosition) < 0.05
      && this.controls.target.distanceTo(this.desiredTarget) < 0.05) return;

    if (this.reducedMotion) {
      const plan = planAutomaticCameraPath(
        this.cameraPoint(this.camera.position), this.cameraPoint(this.desiredPosition), this.cameraObstacles,
        { terrainY: 0, safetyMarginM: 1.5 },
      );
      if (plan.recoveryPoint) this.camera.position.set(plan.recoveryPoint.x, plan.recoveryPoint.y, plan.recoveryPoint.z);
      this.desiredPosition.set(
        plan.correctedDestination.x, plan.correctedDestination.y, plan.correctedDestination.z,
      );
      this.camera.position.set(plan.correctedDestination.x, plan.correctedDestination.y, plan.correctedDestination.z);
      this.controls.target.copy(this.desiredTarget);
      this.automaticRoute = null;
      this.controls.update();
      const actual = this.cameraPoint(this.camera.position);
      if (cameraPathClearance(actual, actual, this.cameraObstacles, { terrainY: 0, safetyMarginM: 1.5 }).collides) {
        // Reduced motion permits a static cut across the scene, but the final
        // camera point must still clear the terrain and building envelopes.
        this.camera.position.copy(this.desiredPosition);
        this.controls.target.copy(this.desiredTarget);
      }
      this.markAutomaticPoseSettled();
      return;
    }

    const followThreshold = this.followId === null ? 0.75 : 12;
    const targetThreshold = this.followId === null ? 0.75 : 12;
    const existing = this.automaticRoute;
    if (existing && (existing.obstacleRevision !== this.obstacleRevision
      || existing.requestedPosition.distanceTo(this.desiredPosition) > followThreshold
      || existing.requestedTarget.distanceTo(this.desiredTarget) > targetThreshold)) {
      this.automaticRoute = null;
      this.settledAutomaticPose = null;
    }
    if (!this.automaticRoute) {
      this.settledAutomaticPose = null;
      const plan = planAutomaticCameraPath(
        this.cameraPoint(this.camera.position), this.cameraPoint(this.desiredPosition), this.cameraObstacles,
        { terrainY: 0, safetyMarginM: 1.5 },
      );
      if (plan.recoveryPoint) {
        // Explicit one-time recovery for a camera newly enclosed by a changed
        // world snapshot; all planned segments begin from this clear pose.
        this.camera.position.set(plan.recoveryPoint.x, plan.recoveryPoint.y, plan.recoveryPoint.z);
      }
      if (plan.correctedDestination.y !== this.desiredPosition.y
        || plan.correctedDestination.x !== this.desiredPosition.x
        || plan.correctedDestination.z !== this.desiredPosition.z) {
        this.desiredPosition.set(plan.correctedDestination.x, plan.correctedDestination.y, plan.correctedDestination.z);
      }
      this.automaticRoute = {
        waypoints: plan.waypoints.map((waypoint) => new THREE.Vector3(waypoint.x, waypoint.y, waypoint.z)),
        index: 0,
        requestedPosition: this.desiredPosition.clone(),
        requestedTarget: this.desiredTarget.clone(),
        obstacleRevision: this.obstacleRevision,
        lifted: plan.kind === "lifted",
      };
    }

    const route = this.automaticRoute;
    if (!route) return;
    const previousPosition = this.camera.position.clone();
    const previousTarget = this.controls.target.clone();
    const previousQuaternion = this.camera.quaternion.clone();
    const secondsPerLeg = Math.max(0.1, seconds);
    const alpha = 1 - Math.exp(-Math.max(0, Math.min(dt, 0.1)) / secondsPerLeg);
    const waypoint = route.waypoints[route.index];
    const nextPosition = this.camera.position.clone().lerp(waypoint, alpha);
    this.controls.target.lerp(this.desiredTarget, alpha);
    this.controls.target.x = THREE.MathUtils.clamp(this.controls.target.x, -WORLD_LIMIT, WORLD_LIMIT);
    this.controls.target.z = THREE.MathUtils.clamp(this.controls.target.z, -WORLD_LIMIT, WORLD_LIMIT);

    this.camera.position.copy(nextPosition);
    let completed = false;
    if (this.camera.position.distanceTo(waypoint) <= 0.35) {
      this.camera.position.copy(waypoint);
      route.index++;
      if (route.index >= route.waypoints.length) {
        this.controls.target.copy(this.desiredTarget);
        this.automaticRoute = null;
        completed = true;
      }
    }
    if (!this.updateAutomaticControls(previousPosition, previousTarget, previousQuaternion)) return;
    if (completed) this.markAutomaticPoseSettled();
  }

  /** Run OrbitControls with relaxed automatic-travel limits, then check its actual output. */
  private updateAutomaticControls(
    previousPosition: THREE.Vector3 = this.camera.position.clone(),
    previousTarget: THREE.Vector3 = this.controls.target.clone(),
    previousQuaternion: THREE.Quaternion = this.camera.quaternion.clone(),
  ): boolean {
    const { minDistance, maxDistance, minPolarAngle, maxPolarAngle } = this.controls;
    this.controls.minDistance = 0;
    this.controls.maxDistance = 1_200;
    this.controls.minPolarAngle = 0;
    this.controls.maxPolarAngle = Math.PI;
    this.controls.update();
    this.controls.minDistance = minDistance;
    this.controls.maxDistance = maxDistance;
    this.controls.minPolarAngle = minPolarAngle;
    this.controls.maxPolarAngle = maxPolarAngle;

    if (!cameraPathClearance(
      this.cameraPoint(previousPosition), this.cameraPoint(this.camera.position), this.cameraObstacles,
      { terrainY: 0, safetyMarginM: 1.5 },
    ).collides) return true;

    // Do not run controls.update again here: it could reapply the same clamp.
    this.camera.position.copy(previousPosition);
    this.controls.target.copy(previousTarget);
    this.camera.quaternion.copy(previousQuaternion);
    this.camera.updateMatrixWorld(true);
    this.automaticRoute = null;
    this.skylineTransition = null;
    this.settledAutomaticPose = null;
    return false;
  }

  private markAutomaticPoseSettled(): void {
    this.settledAutomaticPose = {
      position: this.desiredPosition.clone(),
      target: this.desiredTarget.clone(),
      obstacleRevision: this.obstacleRevision,
    };
  }

  private selectNextAnchor(snap = false): void {
    if (!this.anchors.length) return;
    const world = this.currentWorld;
    const night = Boolean(world && (world.minute_of_day >= 21 * 60 || world.minute_of_day < 5 * 60));
    const rainy = world?.weather.kind === "rain" && world.weather.intensity > 0.2;
    let best: Anchor | null = null;
    let bestScore = -Infinity;
    for (const anchor of this.anchors) {
      if (anchor.id === this.selectedAnchor?.id && this.anchors.length > 1) continue;
      const visit = this.lastVisit.get(anchor.id) ?? -3600;
      const recency = Math.max(-1.4, Math.min(0, (this.elapsed - visit - 50) / 180));
      const timeBonus = (night && anchor.suitability === "night" ? 1.5 : 0) + (rainy && anchor.suitability === "rain" ? 1.2 : 0);
      const score = anchor.importance + anchor.activity * 0.35 + recency + timeBonus;
      if (score > bestScore) { best = anchor; bestScore = score; }
    }
    if (!best) return;
    if (this.selectedAnchor) this.lastVisit.set(this.selectedAnchor.id, this.elapsed);
    this.selectedAnchor = best;
    this.lastVisit.set(best.id, this.elapsed);
    this.holdSeconds = 0;
    this.holdDuration = 45 + this.hash(best.id) % 46;
    this.blendSeconds = this.reducedMotion ? 0.5 : 5.5;
    this.setAnchorPose(best, this.anchorDistance(best));
    if (snap) {
      this.skylineTransition = null;
      this.controls.target.copy(this.desiredTarget);
      this.camera.position.copy(this.desiredPosition);
      this.controls.update();
    } else if (best.framePoints) {
      this.beginSkylineTransition(best);
    }
  }

  private beginSkylineTransition(anchor: Anchor): void {
    if (!anchor.framePoints?.length || this.reducedMotion) {
      this.skylineTransition = null;
      return;
    }
    const highestRoof = Math.max(...anchor.framePoints.map((point) => point.y));
    const startPosition = this.camera.position.clone();
    const startTarget = this.controls.target.clone();
    const midpoint = startTarget.clone().lerp(this.desiredTarget, 0.5);
    midpoint.addScaledVector(anchor.cameraDirection ?? WATCH_DIRECTION, Math.max(24, (anchor.frameRadius ?? 50) * 0.28));
    // Take a gentle aerial arc over the actual roofline, avoiding a straight
    // camera chord through the dense foreground blocks on the way to the shot.
    const minimumApex = highestRoof + Math.max(48, (anchor.frameRadius ?? 50) * 0.55);
    const controlY = Math.max(minimumApex, 2 * minimumApex - 0.5 * (startPosition.y + this.desiredPosition.y));
    const controlPosition = new THREE.Vector3(midpoint.x, controlY, midpoint.z);
    this.skylineTransition = {
      elapsed: 0,
      duration: Math.max(0.5, this.blendSeconds),
      startPosition,
      startTarget,
      controlPosition,
    };
  }

  private setAnchorPose(anchor: Anchor, distance: number): void {
    this.desiredTarget.set(anchor.x, anchor.targetY ?? 0, anchor.z);
    this.desiredPosition.copy(this.desiredTarget).addScaledVector(anchor.cameraDirection ?? WATCH_DIRECTION, distance);
  }

  private updateFollow(dt: number): void {
    if (this.followId === null) return;
    if (!this.followPositionValid && this.findEntityPoint(this.followId, this.followPosition)) {
      this.followPositionValid = true;
      this.followLostSeconds = 0;
    }
    if (!this.followPositionValid) {
      this.followLostSeconds += dt;
      if (this.followLostSeconds >= 4) {
        this.followId = null;
        this.desiredPosition.copy(this.camera.position);
        this.desiredTarget.copy(this.controls.target);
        return;
      }
    }
    const point = this.followPositionValid ? this.followPosition : this.controls.target;
    const lookAhead = this.followKind === 2 ? 10 : this.followKind === 3 ? 8 : 6;
    const forward = this.headingForward(this.followHeading, this.followForward);
    this.desiredTarget.set(
      point.x + forward.x * lookAhead,
      point.y,
      point.z + forward.z * lookAhead,
    );
    if(this.followKind===2){
      const side=new THREE.Vector3(-forward.z,0,forward.x);
      if(side.dot(this.followViewDirection)<0)side.negate();
      this.followViewDirection.set(side.x,.32,side.z).normalize();
    }else this.followViewDirection.copy(WATCH_DIRECTION);
    const minimum=this.followKind===2?145:95;
    const distance=THREE.MathUtils.clamp(Math.max(minimum,this.fitDistance(this.followKind===2?26:18,1.17,this.followViewDirection)),minimum,320);
    this.desiredPosition.copy(this.desiredTarget).addScaledVector(this.followViewDirection, distance);
  }

  private updateFootprintReveal(dt: number): void {
    if (!this.currentWorld || this.revealSeconds > 0) return;
    const threshold = this.baselineFootprint * 1.12;
    if (this.extent.span < threshold || this.elapsed - this.lastRevealAt < 60) {
      this.growthAge = 0;
      return;
    }
    this.growthAge += dt;
    if (this.growthAge < 10) return;
    this.growthAge = 0;
    this.baselineFootprint = this.extent.span;
    this.lastRevealAt = this.elapsed;
    this.revealSeconds = 36;
    this.blendSeconds = 9;
    const size = Math.min(this.extent.span, 400);
    this.desiredTarget.set(this.extent.x, 0, this.extent.z);
    this.desiredPosition.copy(this.desiredTarget).addScaledVector(OVERVIEW_DIRECTION, THREE.MathUtils.clamp(230 + size * 0.7, 300, 520));
  }

  private restoreSelectedAnchor(): void {
    if (this.selectedAnchor) this.setAnchorPose(this.selectedAnchor, this.anchorDistance(this.selectedAnchor));
    this.blendSeconds = 5.5;
  }

  /** Engine heading is atan2(dz, dx): zero points east (+X), π/2 north (+Z). */
  private headingForward(heading: number, out: THREE.Vector3): THREE.Vector3 {
    return out.set(Math.cos(heading), 0, Math.sin(heading));
  }

  private anchorDistance(anchor: Anchor): number {
    let minimum: number;
    let maximum: number;
    let defaultRadius: number;
    if (anchor.id.startsWith("skyline")) {
      if (anchor.framePoints?.length) {
        return this.fitSkylineDistance(anchor.framePoints, anchor.cameraDirection ?? WATCH_DIRECTION,
          new THREE.Vector3(anchor.x, anchor.targetY ?? 0, anchor.z));
      }
      const radius = anchor.frameRadius ?? 90;
      return THREE.MathUtils.clamp(this.fitSphereDistance(radius, 1.12), 105, 360);
    }
    else if (anchor.id.startsWith("crossing:") || anchor.id === "waterfront") { minimum = 270; maximum = 380; defaultRadius = 86; }
    else if (anchor.id.startsWith("founder:")) { minimum = 78; maximum = 170; defaultRadius = 26; }
    else if (anchor.id.startsWith("station")) { minimum = 205; maximum = 300; defaultRadius = 50; }
    else if (anchor.id.startsWith("district:")) { minimum = 160; maximum = 205; defaultRadius = 46; }
    else { minimum = 190; maximum = 290; defaultRadius = 42; }
    const radius = anchor.frameRadius ?? defaultRadius;
    return THREE.MathUtils.clamp(Math.max(minimum, this.fitDistance(radius, 1.17)), minimum, maximum);
  }

  private fitDistance(radius: number, margin: number, direction = WATCH_DIRECTION): number {
    const verticalTangent = Math.tan(THREE.MathUtils.degToRad(this.camera.fov * 0.5));
    const horizontalTangent = verticalTangent * this.validAspect();
    const elevation = THREE.MathUtils.clamp(direction.y, 0.1, 0.95);
    const horizontalDistance = radius * Math.sqrt(1 - elevation * elevation) / Math.max(0.01, horizontalTangent);
    const verticalDistance = radius * elevation / Math.max(0.01, verticalTangent);
    return Math.max(horizontalDistance, verticalDistance) * margin;
  }

  /** Fit a 3D subject, including its height, instead of treating it as a ground-plane map. */
  private fitSphereDistance(radius: number, margin: number): number {
    const verticalTangent = Math.tan(THREE.MathUtils.degToRad(this.camera.fov * 0.5));
    const horizontalTangent = verticalTangent * this.validAspect();
    return radius * margin / Math.max(0.01, Math.min(verticalTangent, horizontalTangent));
  }

  /** Fit every true 3D corner against the perspective frustum, including near-side depth. */
  private fitSkylineDistance(points: THREE.Vector3[], direction: THREE.Vector3, target: THREE.Vector3): number {
    const verticalTangent = Math.tan(THREE.MathUtils.degToRad(this.camera.fov * 0.5));
    const horizontalTangent = verticalTangent * this.validAspect();
    const view = direction.clone().negate().normalize();
    const right = new THREE.Vector3().crossVectors(view, UP).normalize();
    const cameraUp = new THREE.Vector3().crossVectors(right, view).normalize();
    const sideLimit = 0.84;
    // Leave room for the masthead and the bottom controls/caption.
    const verticalLimit = 0.68;
    let distance = 45;
    for (const point of points) {
      const offset = point.clone().sub(target);
      const towardCamera = offset.dot(direction);
      const horizontal = Math.abs(offset.dot(right));
      const vertical = Math.abs(offset.dot(cameraUp));
      distance = Math.max(
        distance,
        towardCamera + horizontal / Math.max(0.01, horizontalTangent * sideLimit),
        towardCamera + vertical / Math.max(0.01, verticalTangent * verticalLimit),
      );
    }
    // Small air around the exact frustum solution keeps antialiasing and skyline edges clear.
    return Math.min(600, distance * 1.045 + 3);
  }

  private validAspect(): number {
    return Number.isFinite(this.camera.aspect) && this.camera.aspect > 0 ? this.camera.aspect : 1;
  }

  private refreshFramingForAspect(): void {
    const aspect = this.validAspect();
    if (Math.abs(aspect - this.framingAspect) < 0.06) return;
    this.framingAspect = aspect;
    if (this.followId !== null) return;
    if (this.currentMode === "overview") {
      this.frameOverview(this.blendSeconds);
    } else if (this.currentMode === "watch" && this.selectedAnchor && this.revealSeconds === 0) {
      this.setAnchorPose(this.selectedAnchor, this.anchorDistance(this.selectedAnchor));
    }
  }

  private frameOverview(seconds: number): void {
    this.desiredTarget.set(this.extent.x, 0, this.extent.z);
    const distance = THREE.MathUtils.clamp(Math.max(320, this.fitDistance(this.extent.span * 0.55, 1.15, OVERVIEW_DIRECTION)), 320, 620);
    this.desiredPosition.copy(this.desiredTarget).addScaledVector(OVERVIEW_DIRECTION, distance);
    this.blendSeconds = this.reducedMotion ? 0.5 : seconds;
    this.overviewInitialized = true;
  }

  private buildAnchors(world: World): Anchor[] {
    const anchors: Anchor[] = [];
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const add = (anchor: Anchor) => {
      if (anchors.some((candidate) => candidate.id === anchor.id)) return;
      anchors.push(anchor);
    };
    const buildings = world.buildings;

    for (const building of buildings) {
      if (building.anchor) {
        const river = this.closestRiverPoint(world.terrain.river, building);
        const targetX = river ? building.x * 0.6 + river.x * 0.4 : building.x;
        const targetZ = river ? building.z * 0.6 + river.z * 0.4 : building.z;
        // Keep the founder as the opening subject; a small riverward offset
        // lets the water read as context without using its length to set scale.
        add({
          id: `founder:${building.id}`,
          label: building.name || "Founding street",
          x: building.x * 0.92 + targetX * 0.08,
          z: building.z * 0.92 + targetZ * 0.08,
          frameRadius: this.founderFrameRadius(building),
          importance: 10,
          activity: building.business_active === true ? 4 : building.occupancy,
          suitability: "all",
        });
      }
      if (/station/i.test(building.kind)) {
        add({ id: `station:${building.id}`, label: "Station street", x: building.x, z: building.z, importance: 8, activity: building.occupancy + 3, suitability: "night" });
      }
    }

    for (const road of world.roads) {
      if (road.stage !== "open" || !road.connected) continue;
      if (road.kind === "bridge") {
        const a = nodes.get(road.from);
        const b = nodes.get(road.to);
        if (a && b) add({ id: `crossing:${road.id}`, label: "River crossing", x: (a.x + b.x) / 2, z: (a.z + b.z) / 2, importance: 8, activity: road.lanes, suitability: "rain" });
      }
    }

    const connectedNodes = new Set<number>();
    const parcelAccess = new Map(world.parcels.map((parcel) => [parcel.id, parcel.access_node]));
    for (const road of world.roads) {
      if (road.stage === "open" && road.connected) { connectedNodes.add(road.from); connectedNodes.add(road.to); }
    }
    for (const project of world.projects) {
      if (project.progress >= 1) continue;
      const point = this.projectPoint(project, world, nodes);
      if (point) {
        const station = project.kind === "station";
        add({
          id: `${station ? "station-project" : "construction"}:${project.id}`,
          label: station ? "Station" : "Construction",
          x: point.x,
          z: point.z,
          importance: station ? 8.5 : 6.5,
          activity: 5,
          suitability: station ? "night" : "rain",
        });
      }
    }

    const neighborhoodBins = new Map<string, { x: number; z: number; count: number; activity: number; minX: number; maxX: number; minZ: number; maxZ: number }>();
    for (const building of buildings) {
      const access = parcelAccess.get(building.parcel_id);
      if (building.stage !== "open" || (connectedNodes.size > 0 && (access === undefined || !connectedNodes.has(access)))) continue;
      const gx = Math.floor(building.x / 120);
      const gz = Math.floor(building.z / 120);
      const id = `${gx}:${gz}`;
      const bin = neighborhoodBins.get(id) ?? {
        x: 0, z: 0, count: 0, activity: 0,
        minX: building.x, maxX: building.x, minZ: building.z, maxZ: building.z,
      };
      bin.x += building.x;
      bin.z += building.z;
      bin.count++;
      bin.activity += building.business_active === true ? 4 : building.occupancy;
      bin.minX = Math.min(bin.minX, building.x); bin.maxX = Math.max(bin.maxX, building.x);
      bin.minZ = Math.min(bin.minZ, building.z); bin.maxZ = Math.max(bin.maxZ, building.z);
      neighborhoodBins.set(id, bin);
    }
    for (const [id, bin] of neighborhoodBins) {
      if (!anchors.some((anchor) => anchor.id.startsWith("founder:") && Math.hypot(anchor.x - bin.x / bin.count, anchor.z - bin.z / bin.count) < 45)) {
        add({
          id: `district:${id}`,
          label: "Neighborhood street",
          x: bin.x / bin.count,
          z: bin.z / bin.count,
          frameRadius: Math.max(bin.maxX - bin.minX, bin.maxZ - bin.minZ) / 2 + 12,
          importance: 5.5,
          activity: bin.activity,
          suitability: "all",
        });
      }
    }

    const accessibleNodes = world.nodes.filter((node) => connectedNodes.has(node.id));
    const riverPoint = this.nearbyRiverAccess(world.terrain.river, accessibleNodes);
    if (riverPoint) add({ id: "waterfront", label: "Waterfront", x: riverPoint.x, z: riverPoint.z, importance: 4.8, activity: world.stats.active_travelers * 0.02, suitability: "rain" });

    const skyline = this.buildSkylineAnchor(world, connectedNodes, parcelAccess);
    if (skyline) add(skyline);

    anchors.sort((a, b) => b.importance + b.activity * 0.2 - (a.importance + a.activity * 0.2));
    const bounded = anchors.slice(0, MAX_ANCHORS);
    // Keep one real skyline composition available in a mature city even when
    // station, crossing and district anchors fill the rest of the budget.
    if (skyline && !bounded.includes(skyline)) bounded[MAX_ANCHORS - 1] = skyline;
    return bounded;
  }

  /** Aim from a real nearby street or river edge toward an existing tall-building cluster. */
  private buildSkylineAnchor(world: World, connectedNodes: Set<number>, parcelAccess: Map<number, number>): Anchor | null {
    const buildings = world.buildings.filter((building) => {
      if (building.stage !== "open") return false;
      const access = parcelAccess.get(building.parcel_id);
      return connectedNodes.size === 0 || access !== undefined && connectedNodes.has(access);
    });
    if (buildings.length === 0) return null;

    let best: { x: number; z: number; floors: number; spread: number; score: number; towers: World["buildings"]; frameBuildings: World["buildings"] } | null = null;
    for (const seed of buildings) {
      const local = buildings.filter((building) => Math.hypot(building.x - seed.x, building.z - seed.z) <= 90);
      const floors = Math.max(...local.map((building) => building.floors));
      if (floors < 5) continue;
      const towers = local.filter((building) => building.floors >= Math.max(5, floors * 0.68));
      if (towers.length === 0) continue;
      const totalWeight = towers.reduce((sum, building) => sum + Math.max(1, building.floors), 0);
      const x = towers.reduce((sum, building) => sum + building.x * Math.max(1, building.floors), 0) / totalWeight;
      const z = towers.reduce((sum, building) => sum + building.z * Math.max(1, building.floors), 0) / totalWeight;
      const spread = Math.max(...towers.map((building) => Math.hypot(building.x - x, building.z - z)
        + Math.hypot(building.width, building.depth) / 2));
      const score = floors * 3 + towers.length * 1.2 + Math.min(local.length, 18) * 0.15;
      // Include adjacent mid-rise roofs that can dominate screen area when
      // they sit closer to the river camera than the tallest tower.
      const frameBuildings = local.filter((building) => building.floors >= Math.max(5, floors * 0.4));
      if (!best || score > best.score) best = { x, z, floors, spread, score, towers, frameBuildings };
    }
    if (!best) return null;

    // Only the selected local tall-building cluster is framed; distant
    // neighborhoods never inflate the shot.

    let cameraSide: Point | null = null;
    let sideDistance = Infinity;
    for (const point of world.terrain.river) {
      const distance = Math.hypot(point.x - best.x, point.z - best.z);
      if (distance < sideDistance) { sideDistance = distance; cameraSide = point; }
    }
    if (sideDistance > 190) cameraSide = null;

    if (!cameraSide) {
      const nearbyStreet = buildings
        .filter((building) => building.floors < best!.floors * 0.65)
        .map((building) => ({ building, distance: Math.hypot(building.x - best!.x, building.z - best!.z) }))
        .filter(({ distance }) => distance >= 16 && distance <= 150)
        .sort((a, b) => a.distance - b.distance)[0];
      if (nearbyStreet) cameraSide = nearbyStreet.building;
    }
    if (!cameraSide) {
      const neighboringBuildings = buildings.filter((building) => Math.hypot(building.x - best!.x, building.z - best!.z) > 35);
      if (neighboringBuildings.length > 0) {
        cameraSide = {
          x: neighboringBuildings.reduce((sum, building) => sum + building.x, 0) / neighboringBuildings.length,
          z: neighboringBuildings.reduce((sum, building) => sum + building.z, 0) / neighboringBuildings.length,
        };
      }
    }
    if (!cameraSide || Math.hypot(cameraSide.x - best.x, cameraSide.z - best.z) < 8) return null;

    const horizontal = new THREE.Vector2(cameraSide.x - best.x, cameraSide.z - best.z).normalize();
    const direction = new THREE.Vector3(horizontal.x, 0.38, horizontal.y).normalize();
    const framePoints = this.skylineBounds(best.frameBuildings);
    const view = direction.clone().negate().normalize();
    const right = new THREE.Vector3().crossVectors(view, UP).normalize();
    const cameraUp = new THREE.Vector3().crossVectors(right, view).normalize();
    let minRight = Infinity, maxRight = -Infinity;
    let minUp = Infinity, maxUp = -Infinity;
    let minDepth = Infinity, maxDepth = -Infinity;
    for (const point of framePoints) {
      const r = point.dot(right), u = point.dot(cameraUp), d = point.dot(direction);
      minRight = Math.min(minRight, r); maxRight = Math.max(maxRight, r);
      minUp = Math.min(minUp, u); maxUp = Math.max(maxUp, u);
      minDepth = Math.min(minDepth, d); maxDepth = Math.max(maxDepth, d);
    }
    const center = right.multiplyScalar((minRight + maxRight) / 2)
      .addScaledVector(cameraUp, (minUp + maxUp) / 2)
      .addScaledVector(direction, (minDepth + maxDepth) / 2);
    const frameRadius = Math.max(...framePoints.map((point) => point.distanceTo(center)));
    const station = world.nodes
      .filter((node) => node.kind === "station" && (connectedNodes.size === 0 || connectedNodes.has(node.id)))
      .sort((a, b) => Math.hypot(a.x - best!.x, a.z - best!.z) - Math.hypot(b.x - best!.x, b.z - best!.z))[0];
    const stationNearby = station && Math.hypot(station.x - best.x, station.z - best.z) <= 150;
    return {
      id: stationNearby ? `skyline:station:${station!.id}` : "skyline:district",
      label: stationNearby ? "Station skyline" : "Neighborhood skyline",
      x: center.x,
      z: center.z,
      targetY: center.y,
      cameraDirection: direction,
      frameRadius,
      framePoints,
      importance: stationNearby ? 7.4 : 6.7,
      activity: Math.min(10, world.stats.active_travelers * 0.02),
      suitability: "night",
    };
  }

  /** Match the visible footprint and rooftop equipment produced by buildingKit. */
  private skylineBounds(buildings: World["buildings"]): THREE.Vector3[] {
    const points: THREE.Vector3[] = [];
    for (const building of buildings) {
      const tower = building.floors >= 12 && (building.kind === "office" || building.kind === "apartment");
      const width = Math.max(6, building.width * (building.anchor ? 0.66 : 0.84)) + (tower ? 0.65 : 1.2);
      // Tower entry canopies project about 2m beyond the street facade.
      const depth = Math.max(6, building.depth * 0.84) + (tower ? 4.1 : 1.2);
      const cos = Math.cos(building.rotation), sin = Math.sin(building.rotation);
      const base = 0.58 + building.floors * 3.25;
      // Flat roofs carry tanks/screens; 12+ floor towers add a screened crown
      // and a slim aerial reaching just under four meters above their floor count.
      const roofTop = base + (tower ? 4.05 : 2.75);
      for (const localX of [-width / 2, width / 2]) {
        for (const localZ of [-depth / 2, depth / 2]) {
          const x = building.x + cos * localX + sin * localZ;
          const z = building.z - sin * localX + cos * localZ;
          points.push(new THREE.Vector3(x, 0, z), new THREE.Vector3(x, roofTop, z));
        }
      }
    }
    return points;
  }

  private projectPoint(project: Project, world: World, nodes: Map<number, Point>): Point | null {
    if (project.parcel_id !== null) {
      const parcel = world.parcels.find((candidate) => candidate.id === project.parcel_id);
      if (parcel) return parcel;
    }
    if (project.from !== null && project.to !== null) {
      const a = nodes.get(project.from);
      const b = nodes.get(project.to);
      if (a && b) return { x: (a.x + b.x) / 2, z: (a.z + b.z) / 2 };
    }
    return null;
  }

  private nearbyRiverAccess(river: Point[], nodes: Point[]): Point | null {
    if (!river.length || !nodes.length) return null;
    const mid = river[Math.floor(river.length / 2)];
    let best: Point | null = null;
    let bestDistance = Infinity;
    for (const node of nodes) {
      const distance = Math.hypot(node.x - mid.x, node.z - mid.z);
      if (distance < bestDistance) { bestDistance = distance; best = node; }
    }
    return bestDistance <= 180 ? best : null;
  }

  private closestRiverPoint(river: Point[], point: Point): Point | null {
    if (river.length === 0) return null;
    let closestX = river[0].x;
    let closestZ = river[0].z;
    let closestDistance = Infinity;
    for (let index = 0; index < river.length - 1; index++) {
      const a = river[index];
      const b = river[index + 1];
      const dx = b.x - a.x;
      const dz = b.z - a.z;
      const lengthSquared = dx * dx + dz * dz;
      const t = lengthSquared > 0
        ? THREE.MathUtils.clamp(((point.x - a.x) * dx + (point.z - a.z) * dz) / lengthSquared, 0, 1)
        : 0;
      const x = a.x + dx * t;
      const z = a.z + dz * t;
      const distance = Math.hypot(point.x - x, point.z - z);
      if (distance < closestDistance) {
        closestDistance = distance;
        closestX = x;
        closestZ = z;
      }
    }
    if (river.length === 1) {
      closestX = river[0].x;
      closestZ = river[0].z;
    }
    return { x: closestX, z: closestZ };
  }

  private founderFrameRadius(building: World["buildings"][number]): number {
    const footprintRadius = Math.hypot(building.width, building.depth) / 2;
    // Frame the shop, its frontage, and a little sidewalk. The river is a
    // background cue; its geographic distance must not shrink the focal shop.
    return Math.max(24, footprintRadius + 10);
  }

  private developedExtent(world: World): Extent {
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const connected = new Set<number>();
    const roadDegree = new Map<number, number>();
    const parcelAccess = new Map(world.parcels.map((parcel) => [parcel.id, parcel.access_node]));
    for (const road of world.roads) {
      if (road.stage === "open" && road.connected) {
        connected.add(road.from); connected.add(road.to);
        roadDegree.set(road.from, (roadDegree.get(road.from) ?? 0) + 1);
        roadDegree.set(road.to, (roadDegree.get(road.to) ?? 0) + 1);
      }
    }
    const developedAccess = new Set<number>();
    const points: Point[] = [];
    for (const building of world.buildings) {
      const access = parcelAccess.get(building.parcel_id);
      if (building.anchor || building.stage === "open" && (connected.size === 0 || access !== undefined && connected.has(access))) {
        points.push(building);
        if (access !== undefined) developedAccess.add(access);
      }
    }
    for (const id of connected) {
      const node = nodes.get(id);
      if (node && (developedAccess.has(id) || (roadDegree.get(id) ?? 0) > 1)) points.push(node);
    }
    if (points.length === 0) return { x: 50, z: 20, span: 120 };
    let minX = Infinity; let maxX = -Infinity; let minZ = Infinity; let maxZ = -Infinity;
    let sumX = 0; let sumZ = 0;
    for (const point of points) {
      minX = Math.min(minX, point.x); maxX = Math.max(maxX, point.x);
      minZ = Math.min(minZ, point.z); maxZ = Math.max(maxZ, point.z);
      sumX += point.x; sumZ += point.z;
    }
    return { x: sumX / points.length, z: sumZ / points.length, span: Math.max(120, maxX - minX, maxZ - minZ) };
  }

  private findEntityPoint(id: number, out: THREE.Vector3): boolean {
    const world = this.currentWorld;
    if (!world) return false;
    const resident = this.findResident(id);
    if (resident) {
      if (resident.bus_id !== null && resident.bus_id !== undefined) {
        const bus = (world as FollowWorld).buses?.find((entity) => entity.id === resident.bus_id);
        if (bus && this.busPoint(bus, out)) return true;
      }
      if (resident.train_id !== null && resident.train_id !== undefined) {
        const train = (world as FollowWorld).trains.find((entity) => entity.id === resident.train_id);
        if (train && this.trainPoint(train, out)) return true;
      }
      if (resident.bus_stop_node !== null && resident.bus_stop_node !== undefined) {
        const lines = (world as FollowWorld).bus_lines ?? [];
        const busLine = lines.find((line) => line.stops.includes(resident.bus_stop_node!)
          && (resident.bus_origin_stop == null || line.stops.includes(resident.bus_origin_stop))
          && (resident.bus_destination_stop == null || line.stops.includes(resident.bus_destination_stop)));
        const origin = busLine?.stops.indexOf(resident.bus_origin_stop ?? -1) ?? -1;
        const destination = busLine?.stops.indexOf(resident.bus_destination_stop ?? -1) ?? -1;
        const direction = origin >= 0 && destination >= 0 && origin !== destination
          ? destination > origin ? 1 : -1
          : undefined;
        const stop = busLine?.stop_positions?.find((position) => position.node_id === resident.bus_stop_node
          && (direction === undefined || position.direction === direction));
        if (stop) {
          out.set(stop.x, 0, stop.z);
          this.followHeading = stop.heading;
          this.followKind = 0;
          return true;
        }
      }
      if (resident.location_node !== null && resident.location_node !== undefined) {
        const node = world.nodes.find((entity) => entity.id === resident.location_node);
        if (node) {
          out.set(node.x, 0, node.z);
          this.followKind = 0;
          this.followHeading = 0;
          return true;
        }
      }
      const activity = `${resident.activity} ${resident.transfer ?? ""}`.toLowerCase();
      const buildingId = /work|shift|job/.test(activity)
        ? resident.work_building ?? resident.home_building
        : resident.home_building;
      const building = world.buildings.find((entity) => entity.id === buildingId);
      if (building) {
        out.set(building.x, 0, building.z);
        this.followKind = 0;
        this.followHeading = 0;
        return true;
      }
      return false;
    }
    const building = world.buildings.find((entity) => entity.id === id);
    if (building) { out.set(building.x, 0, building.z); return true; }
    const node = world.nodes.find((entity) => entity.id === id);
    if (node) { out.set(node.x, 0, node.z); return true; }
    const road = world.roads.find((entity) => entity.id === id);
    if (road) {
      const a = world.nodes.find((entity) => entity.id === road.from);
      const b = world.nodes.find((entity) => entity.id === road.to);
      if (a && b) { out.set((a.x + b.x) / 2, 0, (a.z + b.z) / 2); return true; }
    }
    const train = world.trains.find((entity) => entity.id === id);
    if (train) {
      if (this.trainPoint(train, out)) return true;
    }
    const bus = (world as FollowWorld).buses?.find((entity) => entity.id === id);
    if (bus && this.busPoint(bus, out)) return true;
    return false;
  }

  private findResident(id: number): FollowResident | null {
    return (this.currentWorld as FollowWorld | null)?.residents?.find((resident) => resident.id === id) ?? null;
  }

  private trainPoint(train: FollowTrain, out: THREE.Vector3): boolean {
    const world = this.currentWorld;
    if (!world || train.route.length < 2) return false;
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    let distance = Math.max(0, train.progress);
    for (let index = 0; index < train.route.length - 1; index++) {
      const from = nodes.get(train.route[index]);
      const to = nodes.get(train.route[index + 1]);
      if (!from || !to) return false;
      const rail = world.roads.find((road) => road.kind === "rail" && road.connected
        && (road.from === from.id && road.to === to.id || road.from === to.id && road.to === from.id));
      const length = rail?.length ?? Math.hypot(to.x - from.x, to.z - from.z);
      const direction = train.direction ?? 1;
      if (distance <= length || index === train.route.length - 2) {
        const progress = THREE.MathUtils.clamp(distance / Math.max(1, length), 0, 1);
        out.set(from.x + (to.x - from.x) * progress, 0, from.z + (to.z - from.z) * progress);
        this.followHeading = Math.atan2(direction >= 0 ? to.z - from.z : from.z - to.z, direction >= 0 ? to.x - from.x : from.x - to.x);
        this.followKind = 2;
        return true;
      }
      distance -= length;
    }
    return false;
  }

  private busPoint(bus: BusView, out: THREE.Vector3): boolean {
    const world = this.currentWorld as FollowWorld | null;
    const line = world?.bus_lines?.find((candidate) => candidate.id === bus.line_id);
    if (!world || !line || line.route.length < 2) return false;
    const nodes = new Map(world.nodes.map((node) => [node.id, node]));
    const segments = line.route.slice(0, -1).map((nodeId, index) => {
      const from = nodes.get(nodeId);
      const to = nodes.get(line.route[index + 1]);
      if (!from || !to) return null;
      const road = world.roads.find((candidate) => candidate.connected && candidate.kind !== "rail"
        && (candidate.from === from.id && candidate.to === to.id || candidate.from === to.id && candidate.to === from.id));
      return { from, to, length: road?.length ?? Math.hypot(to.x - from.x, to.z - from.z) };
    });
    if (segments.some((segment) => segment === null)) return false;
    const route = segments as Array<{ from: Point & { id: number }; to: Point & { id: number }; length: number }>;
    const total = route.reduce((sum, segment) => sum + segment.length, 0);
    if (!(total > 0)) return false;
    let distance = ((Math.max(0, bus.progress) % total) + total) % total;
    for (const segment of route) {
      if (distance <= segment.length) {
        const progress = THREE.MathUtils.clamp(distance / Math.max(1, segment.length), 0, 1);
        out.set(
          segment.from.x + (segment.to.x - segment.from.x) * progress,
          0,
          segment.from.z + (segment.to.z - segment.from.z) * progress,
        );
        this.followHeading = Math.atan2(bus.direction >= 0 ? segment.to.z - segment.from.z : segment.from.z - segment.to.z,
          bus.direction >= 0 ? segment.to.x - segment.from.x : segment.from.x - segment.to.x);
        this.followKind = 3;
        return true;
      }
      distance -= segment.length;
    }
    return false;
  }

  private findEntity(id: number): string | null {
    const world = this.currentWorld;
    if (!world) return null;
    const resident = this.findResident(id);
    if (resident) return resident.name || "Resident";
    const building = world.buildings.find((entity) => entity.id === id);
    if (building) return building.name || "Building";
    const train = world.trains.find((entity) => entity.id === id);
    if (train) return "Train";
    const bus = (world as FollowWorld).buses?.find((entity) => entity.id === id);
    if (bus) return "Bus";
    if (world.nodes.some((entity) => entity.id === id)) return "Street scene";
    if (world.roads.some((entity) => entity.id === id)) return "Road scene";
    return null;
  }

  private hash(value: string): number {
    let result = 2166136261;
    for (let i = 0; i < value.length; i++) result = Math.imul(result ^ value.charCodeAt(i), 16777619);
    return result >>> 0;
  }
}
