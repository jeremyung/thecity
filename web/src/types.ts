// Snapshot types are generated from the Rust engine (web/src/generated, see
// scripts/generate-types.mjs). Change them in Rust, then regenerate.
import type {
  BuildingAccessView,
  BusLineView,
  Event,
  Node,
  RailAlignmentView,
  RailPlatformView,
  RoadDeckView,
  Snapshot,
} from './generated';

export type {
  Building,
  BuildingKind,
  BuildingOriginalFootprint,
  BusLineView,
  BusStopView,
  BusView,
  EventKind,
  NodeKind,
  Parcel,
  Point,
  Project,
  ProjectKind,
  RearExtensionView,
  ResidentView,
  Road,
  RoadKind,
  Stage,
  TrainView,
  WeatherKind,
} from './generated';

export type BuildingAccess = BuildingAccessView;
export type WorldNode = Node;
export type RailAlignment = RailAlignmentView;
export type RoadDeck = RoadDeckView;
export type RailPlatform = RailPlatformView;
export type WorldEvent = Event;
export type BusLine = BusLineView;

/** Fields the engine always sends but isolated rendering fixtures may omit. */
type FixtureOptional =
  | 'presentation_revision'
  | 'road_decks'
  | 'expressway_profile'
  | 'rail_platforms'
  | 'rail_alignments'
  | 'ground'
  | 'rear_extensions'
  | 'building_original_footprints'
  | 'residents'
  | 'buses'
  | 'bus_lines'
  | 'building_access';

/** One engine snapshot as the worker delivers it. */
export type World = Omit<Snapshot, FixtureOptional> & Partial<Pick<Snapshot, FixtureOptional>>;

export type Style = 'anime' | 'miniature';
export interface Metrics {
  tickMs: number;
  memoryBytes: number;
  backlog: number;
  tick: number;
  achievableSpeed: number;
  droppedWallMs: number;
}
export const WORKER_PROTOCOL_VERSION = 5 as const;
export type WorkerInput =
  | {
      type: 'init';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      seed: number;
      save?: string;
    }
  | {
      type: 'control';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      sequence: number;
      paused?: boolean;
      hidden?: boolean;
      speed?: number;
      development?: number;
    }
  | {
      type: 'recycle';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      buffer: ArrayBuffer;
    }
  | {
      type: 'save';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      request: number;
    }
  | {
      type: 'inspect';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      request: number;
      id: number;
    }
  | { type: 'resync'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number };
export type WorkerOutput =
  | { type: 'world'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; world: World }
  | {
      type: 'movement';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      tick: number;
      wordCount: number;
      buffer: ArrayBuffer;
      trainDoors: Float32Array;
      presentationRevision: string;
      minute: number;
      day: number;
    }
  | {
      type: 'metrics';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      metrics: Metrics;
    }
  | {
      type: 'save';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      request: number;
      data: string;
    }
  | {
      type: 'inspect';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      request: number;
      data: unknown;
    }
  | {
      type: 'controlAck';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      appliedSequence: number;
      tick: number;
    }
  | {
      type: 'error';
      protocolVersion: typeof WORKER_PROTOCOL_VERSION;
      epoch: number;
      message: string;
      code?: 'protocol-version';
    };
