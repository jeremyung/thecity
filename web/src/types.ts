export interface Point { x: number; z: number }
/** Fixed door-to-sidewalk segment shared with pedestrian routing. */
export interface BuildingAccess { building_id: number; road_id: number; points: readonly Point[] }
export interface WorldNode extends Point { id: number; kind: string }
/** Engine-owned arc samples: distance, x, z, tangent x, tangent z. */
export interface RailAlignment { road_id: number; samples: [number,number,number,number,number][] }
export interface RoadDeck { road_id: number; deck_height: number; controls: [[number,number],[number,number]] }
/** Engine-owned rail terminal frame; +X=(uz,-ux), +Z=(ux,uz). */
export interface RailPlatform extends Point {
  node_id: number; ux: number; uz: number; surface_y: number;
  stair_x: number; stair_bottom_z: number; stair_top_z: number;
  ground_y: number; stair_steps: number;
  /** 0 site, 1 supports, 2 deck/access, 3 canopy, 4 complete. */
  construction_phase: number;
  /** Engine-owned centerline in the terminal local frame, street to stair foot. */
  ground_path: readonly Point[]; ground_width: number;
}
export interface Road { id: number; from: number; to: number; kind: string; stage: string; progress: number; connected: boolean; length: number; lanes: number }
export interface Parcel extends Point { id: number; width: number; depth: number; rotation?: number; access_node: number; suitability: number; protected: boolean; reservation: number | null; /** Open space kept on a protected lot. */ open_space?: 'park' | 'shrine' | 'plaza' }
export interface Building extends Point { id: number; parcel_id: number; rotation: number; width: number; depth: number; floors: number; kind: string; stage: string; progress: number; capacity: number; occupancy: number; business_active?: boolean; built_day: number; name: string; appearance_seed: number; history: string[]; anchor: boolean; vacant_days: number }
export interface Project { id: number; kind: string; from: number | null; to: number | null; parcel_id: number | null; stage: string; progress: number; reason: string; duration: number; started_tick: number }
/** Temporary rear annex view supplied by the engine while its project is active. */
export interface RearExtensionView {
  project_id: number; building_id: number; x: number; z: number; width: number; depth: number;
  rotation: number; floors: number; stage: string; progress: number;
}
/** Persisted pre-extension parcel geometry used to retain the original building mass. */
export interface BuildingOriginalFootprint extends Point {
  building_id: number; width: number; depth: number; rotation: number;
}
export interface WorldEvent { id: number; tick: number; kind: string; entity_id: number; reason: string }
export interface BusView { id: number; line_id: number; progress: number; direction: 1 | -1; dwelling: boolean; passengers: number; capacity: number; next_stop: number | null; waiting: number }
export interface BusLineView { id: number; name?: string; stops: number[]; route: number[]; active: boolean; stop_positions?: { node_id: number; x: number; z: number; heading: number; direction: 1 | -1 }[] }
export interface World {
  schema_version: number; engine_version: string; seed: number; tick: number; day: number; minute_of_day: number; development_rate: number; revision: number; stage: string;
  /** Required on engine snapshots; optional for isolated rendering fixtures. */
  presentation_revision?: string;
  bounds: { min_x: number; max_x: number; min_z: number; max_z: number };
  road_decks?: RoadDeck[];
  expressway_profile?: import('./render/expressway-profile').ExpresswayProfile;
  rail_platforms?: RailPlatform[];
  rail_alignments?: RailAlignment[];
  terrain: { river: Point[]; river_half_width?: number; protected: Point[][]; gateways: number[] };
  weather: { kind: 'clear' | 'overcast' | 'rain'; intensity: number };
  nodes: WorldNode[]; roads: Road[]; parcels: Parcel[]; buildings: Building[]; ground?: { block: number; points: [number, number][] }[];
  projects: Project[];
  rear_extensions?: RearExtensionView[];
  building_original_footprints?: BuildingOriginalFootprint[];
  residents?: {id:number;name:string;home_building:number;work_building:number|null;activity:string;transfer?:string;train_id?:number|null;bus_id?:number|null;bus_origin_stop?:number|null;bus_destination_stop?:number|null;bus_stop_node?:number|null;location_node?:number|null}[];
  trains: { id: number; route: number[]; progress: number; direction?:number; dwelling?:boolean;door_open_fraction?:number;door_terminal?:number|null;passengers?:number;capacity?:number;station_waiting?:number;gateway_waiting?:number }[]; events: WorldEvent[];
  buses?: BusView[]; bus_lines?: BusLineView[];
  building_access?: BuildingAccess[];
  stats: { population: number; active_travelers: number; buildings: number; active_projects: number };
}
export type Style = 'anime' | 'miniature';
export interface Metrics { tickMs: number; memoryBytes: number; backlog: number; tick: number; achievableSpeed: number; droppedWallMs: number }
export const WORKER_PROTOCOL_VERSION = 5 as const;
export type WorkerInput =
  | { type: 'init'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; seed: number; save?: string }
  | { type: 'control'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; sequence: number; paused?: boolean; hidden?: boolean; speed?: number; development?: number }
  | { type: 'recycle'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; buffer: ArrayBuffer }
  | { type: 'save'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; request: number }
  | { type: 'inspect'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; request: number; id: number }
  | { type: 'resync'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number };
export type WorkerOutput =
  | { type: 'world'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; world: World }
  | { type: 'movement'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; tick: number; wordCount: number; buffer: ArrayBuffer; trainDoors: Float32Array; presentationRevision: string; minute: number; day: number }
  | { type: 'metrics'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; metrics: Metrics }
  | { type: 'save'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; request: number; data: string }
  | { type: 'inspect'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; request: number; data: unknown }
  | { type: 'controlAck'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; appliedSequence: number; tick: number }
  | { type: 'error'; protocolVersion: typeof WORKER_PROTOCOL_VERSION; epoch: number; message: string; code?: 'protocol-version' };
