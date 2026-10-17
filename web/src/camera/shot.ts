import * as THREE from 'three';
import { type World } from '../types';

export type StreetSubject = {
  x: number;
  z: number;
  roadId: number;
  forward: { x: number; z: number };
};

export type Anchor = {
  id: string;
  label: string;
  x: number;
  z: number;
  targetY?: number;
  streetForward?: { x: number; z: number };
  streetRoadId?: number;
  streetSubjects?: StreetSubject[];
  /** World-space direction from the framed subject toward the camera. */
  cameraDirection?: THREE.Vector3;
  frameRadius?: number;
  /** Actual rotated facade/roof corners for perspective-safe skyline framing. */
  framePoints?: THREE.Vector3[];
  importance: number;
  activity: number;
  suitability: 'all' | 'night' | 'rain';
};

export type Extent = { x: number; z: number; span: number };

export const MAX_ANCHORS = 32;

export const WORLD_LIMIT = 430;

export const MIN_HEIGHT = 12;

export const UP = new THREE.Vector3(0, 1, 0);

// Keep enough sky and waterfront in frame to read the city as a place, while
// retaining the higher angle needed to understand its street fabric.
export const WATCH_DIRECTION = new THREE.Vector3(0.65, 0.28, 0.7).normalize();

export const OVERVIEW_DIRECTION = new THREE.Vector3(0.65, 0.42, 0.7).normalize();

export type SkylineCluster = {
  x: number;
  z: number;
  floors: number;
  spread: number;
  score: number;
  towers: World['buildings'];
  frameBuildings: World['buildings'];
};
