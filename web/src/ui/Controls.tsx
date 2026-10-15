import { useEffect, useMemo, useRef, useState, type MouseEvent } from 'react';
import {
  AudioLines,
  Bookmark,
  ChevronDown,
  CircleHelp,
  Clock3,
  Eye,
  Focus,
  Headphones,
  Map,
  Pause,
  Play,
  RotateCcw,
  SlidersHorizontal,
  Volume2,
  VolumeX,
  X,
} from 'lucide-react';
import type { Building, Style, World } from '../types';
import { LENS_PRESETS, type LensPresetId } from '../camera/lens';
import './style.css';
import ReviewRecording from './ReviewRecording';
import ReviewCameraPose, { type ApplyReviewPose } from './ReviewCameraPose';

export interface ControlsProps {
  world: World | null;
  ready: boolean;
  error: string | null;
  message: string | null;
  paused: boolean;
  speed: number;
  achievableSpeed?: number | null;
  development: number;
  style: Style;
  preparingStyle?: Style | null;
  mode: 'watch' | 'explore' | 'overview';
  sceneLabel: string;
  preview: number | null;
  sound: boolean;
  musicLevel: number;
  cityLevel: number;
  focus: boolean;
  contrast: 'standard' | 'high';
  lens: LensPresetId;
  selected: Building | null;
  subject?: {
    id: number;
    name: string;
    kind: string;
    activity: string;
    destination?: string;
    destination_name?: string;
    history?: string[];
    passengers?: number;
    capacity?: number;
    station_waiting?: number;
    gateway_waiting?: number;
    line?: string;
    nextStop?: string;
    dwelling?: boolean;
    waiting?: number;
  } | null;
  quality?: 'high' | 'balanced';
  metrics: { timing?: { samples:number;phase:'loading'|'ready'|'hidden';frameP95Ms:number;cpuP95Ms:number;maxFrameMs:number }; fps: number; frameMs: number; calls: number; triangles: number; textures: number; geometries: number; backend: string; pendingBuildings?: number; pendingChunks?: number; pendingSidewalks?: number; pendingRoads?: number; styleSwitch?: { target:string; samples:number; compiledSamples:number; maxSubmitMs:number; prepareMs:number; submitMs:number; waitMs:number; applyMs:number; outcome:string } } | null;
  onPause: () => void;
  onSpeed: (n: number) => void;
  onDevelopment: (n: number) => void;
  onStyle: (s: Style) => void;
  onMode: (m: 'watch' | 'explore' | 'overview') => void;
  onPreview: (h: number | null) => void;
  onSound: () => void;
  onLevels: (music: number, city: number) => void;
  onFocus: () => void;
  onContrast: (contrast: 'standard' | 'high') => void;
  onLens: (lens: LensPresetId) => void;
  onSave: () => void;
  onExport?: () => void;
  onImport?: (file: File) => void;
  onResume: () => void;
  onNewWorld: (seed: number) => void;
  onCloseInspector: () => void;
  onFollow: (id: number) => void;
  onQuality?: (quality: 'high' | 'balanced') => void;
  onReviewMature?: () => void;
  onReviewStreetLights?: () => boolean;
  onReviewWaterReflection?: () => boolean;
  onReviewSkyline?: () => void;
  onReviewStation?: () => void;
  onReviewBuilding?: (id:number) => void;
  onReadCameraPose?: () => string;
  onApplyCameraPose?: ApplyReviewPose;
  fixedReviewQuality?: boolean;
  onFixedReviewQuality?: (enabled:boolean) => void;
  onReviewWaterfront?: (preset?: 'overhead' | 'low') => void;
  onDismissError?: () => void;
  onGraphicsRetry: () => void;
  onGraphicsFallback: () => void;
}

interface ResidentSummary {
  id: number;
  name: string;
  home_building: number;
  work_building: number | null;
  activity: string;
}

const modeOptions = [
  { value: 'watch', label: 'Watch', icon: Eye, hint: 'Return to the city’s quiet camera direction' },
  { value: 'explore', label: 'Explore', icon: Map, hint: 'Move the camera through the city' },
  { value: 'overview', label: 'Overview', icon: SlidersHorizontal, hint: 'See the whole developed district' },
] as const;

function cityTime(world: World | null) {
  if (!world) return '--:--';
  const wholeMinutes = Math.floor(world.minute_of_day);
  const hours = Math.floor(wholeMinutes / 60) % 24;
  const minutes = wholeMinutes % 60;
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
}

function prettyKind(value: string) {
  return value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function prettyHistory(value: string) {
  const words = value.replaceAll('_', ' ').trim();
  return words ? words[0].toUpperCase() + words.slice(1) : words;
}

const eventKindNames: Record<string, string> = {
  FoundingShopOpened: 'Founding shop opened',
  RoadOpened: 'Street opened',
  BuildingOpened: 'Place opened',
  BusinessReopened: 'Business reopened',
  BusinessClosed: 'Business closed',
  RenovationStarted: 'Renovation began',
  RailOpened: 'Rail link opened',
  StationServiceStarted: 'Station service began',
  RailTripStarted: 'Rail journey began',
  RailReturnQueued: 'Waiting for return train',
  RailWorkStarted: 'Work journey arrived',
  TrainStationStop: 'Train at station',
  TrainGatewayStop: 'Train at gateway',
  TrainPassengerAlighted: 'Train passenger arrived',
  ResidentTripStarted: 'Resident journey began',
  ResidentTripArrived: 'Resident arrived',
  BusServiceStarted: 'Bus service began',
  BusServiceExpanded: 'Bus service expanded',
  BusTripStarted: 'Bus journey began',
  BusStop: 'Bus at stop',
  BusPassengerBoarded: 'Passenger boarded bus',
  BusPassengerAlighted: 'Passenger left bus',
  DeliveryDispatched: 'Delivery departed',
  DeliveryArrived: 'Delivery arrived',
  DeliveryReturned: 'Delivery returned',
};

function eventKindLabel(value: string) {
  const known = eventKindNames[value];
  if (known) return known;
  const words = value.replaceAll('_', ' ').replace(/([a-z0-9])([A-Z])/g, '$1 $2').trim();
  return words ? words[0].toUpperCase() + words.slice(1) : 'City event';
}

export function RecentEvents({ events }: { events: World['events'] }) {
  return <div className="recent-events"><span className="panel-kicker">RECENTLY, HERE</span>{events.length ? <ul>{events.map((event) => <li key={`${event.id}-${event.tick}`}><span className="event-kind">{eventKindLabel(event.kind)}</span><span className="event-reason">{event.reason ? prettyHistory(event.reason) : 'A small change along this street.'}</span></li>)}</ul> : <p className="quiet-empty">The day is still unfolding.</p>}</div>;
}

function stageTitle(value: string) {
  return prettyKind(value.replace(/([a-z])([A-Z])/g, '$1 $2'));
}

function buildingSubtitle(building: Building) {
  const kind = prettyKind(building.kind);
  if (building.kind === 'home' || building.kind === 'apartment') {
    return `${kind} · ${building.occupancy} of ${building.capacity} in residence`;
  }
  if (building.stage === 'open' && ['shop', 'cafe', 'office'].includes(building.kind)
      && typeof building.business_active === 'boolean') {
    return `${kind} · ${building.business_active ? 'Business open' : 'Awaiting a new tenant'}`;
  }
  return `${kind} · ${stageTitle(building.stage)}`;
}

function hourTime(hour: number) {
  return `${String(hour).padStart(2, '0')}:00`;
}

export function ContrastToggle({ contrast, onContrast }: { contrast: 'standard' | 'high'; onContrast: (contrast: 'standard' | 'high') => void }) {
  return <label className="contrast-toggle"><input type="checkbox" checked={contrast === 'high'} onChange={(event) => onContrast(event.currentTarget.checked ? 'high' : 'standard')} /><span>High contrast controls</span></label>;
}

export function LensControl({ lens, onLens }: { lens: LensPresetId; onLens: (lens: LensPresetId) => void }) {
  return <label className="quality-control lens-control"><span>Camera lens</span><select aria-label="Camera lens" value={lens} onChange={(event) => onLens(event.target.value as LensPresetId)}>
    {(Object.values(LENS_PRESETS) as typeof LENS_PRESETS[LensPresetId][]).map((preset) => <option key={preset.id} value={preset.id}>{preset.label}</option>)}
  </select></label>;
}

export function activeBusToFollow(world: World | null) {
  return world?.buses?.find((bus) => world.bus_lines?.some((line) => line.id === bus.line_id && line.active)) ?? null;
}

export function busLineLabel(world: World | null, lineId: number | undefined, reportedName?: string) {
  if (reportedName && !/^\d+$/.test(reportedName)) return reportedName;
  const lines = world?.bus_lines ?? [];
  const line = lines.find((candidate) => candidate.id === lineId);
  if (line?.name) return line.name;
  const index = line ? lines.indexOf(line) : -1;
  return index >= 0 ? `Neighborhood line ${index + 1}` : 'Neighborhood line';
}

export default function Controls(props: ControlsProps) {
  const {
    world, ready, error, message, paused, speed, achievableSpeed, development, style, preparingStyle, mode, sceneLabel, preview, sound,
    musicLevel, cityLevel, focus, contrast, lens, selected, subject, quality, metrics, onPause, onSpeed, onDevelopment,
    onStyle, onMode, onPreview, onSound, onLevels, onFocus, onContrast, onLens, onSave, onResume, onNewWorld,
    onCloseInspector, onFollow, onQuality, onReviewMature, onReviewSkyline, onReviewStation, onReviewWaterfront, onDismissError, onGraphicsRetry, onGraphicsFallback, onExport, onImport,
  } = props;
  const [panel, setPanel] = useState<'sound' | 'world' | 'details' | 'visual' | null>(null);
  const [streetLightsOn, setStreetLightsOn] = useState(true);
  const [rawWaterReflection, setRawWaterReflection] = useState(false);
  const [seedText, setSeedText] = useState('');
  const [reviewBuildingId, setReviewBuildingId] = useState('');
  const reviewBuilding = import.meta.env.DEV && /^\d+$/.test(reviewBuildingId)
    ? world?.buildings.find(building => building.id === Number(reviewBuildingId)) : undefined;
  const [confirmSeed, setConfirmSeed] = useState<number | null>(null);
  const importInput = useRef<HTMLInputElement>(null);
  const panelOpener = useRef<HTMLButtonElement | null>(null);
  const panelHeading = useRef<HTMLHeadingElement | null>(null);
  const focusPanelOnOpen = useRef(false);

  useEffect(() => {
    if (selected) setPanel('details');
  }, [selected?.id]);

  useEffect(() => {
    if (!panel) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') closePanel();
    };
    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [panel]);

  useEffect(() => {
    if (!panel || !focusPanelOnOpen.current) return;
    focusPanelOnOpen.current = false;
    panelHeading.current?.focus();
  }, [panel]);

  const closePanel = () => {
    focusPanelOnOpen.current = false;
    setPanel(null);
    const opener = panelOpener.current;
    if (opener?.isConnected) requestAnimationFrame(() => opener.focus());
  };

  const togglePanel = (next: 'sound' | 'world' | 'details' | 'visual', event: MouseEvent<HTMLButtonElement>) => {
    if (panel === next) {
      closePanel();
      return;
    }
    panelOpener.current = event.currentTarget;
    focusPanelOnOpen.current = true;
    setPanel(next);
  };

  const events = useMemo(() => {
    if (!world) return [];
    const matching = selected
      ? world.events.filter((event) => event.entity_id === selected.id)
      : world.events;
    return matching.slice(-5).reverse();
  }, [selected, world]);

  const residents = useMemo(() => {
    const optionalWorld = world as (World & { residents?: ResidentSummary[] }) | null;
    const all = optionalWorld?.residents ?? [];
    const nearby = selected
      ? all.filter((resident) => resident.home_building === selected.id || resident.work_building === selected.id)
      : all;
    return nearby.slice(0, 3);
  }, [selected, world]);

  const submitNewWorld = () => {
    const value = Number(seedText.trim());
    if (!Number.isSafeInteger(value) || value < 0) return;
    setConfirmSeed(value);
  };

  const startNewWorld = () => {
    if (confirmSeed === null) return;
    onNewWorld(confirmSeed);
    setConfirmSeed(null);
    setSeedText('');
    setPanel(null);
  };

  const clockLabel = world ? `DAY ${String(world.day).padStart(3, '0')}` : 'CITY TIME';
  const shownTime = preview === null ? cityTime(world) : hourTime(preview);
  const displayedHour = preview === null
    ? world ? Math.floor(world.minute_of_day / 60) % 24 : null
    : preview % 24;
  const mastheadTone = displayedHour !== null && (displayedHour >= 19 || displayedHour < 6) ? 'night' : 'day';
  const simulationOverloaded = speed > 1 && achievableSpeed !== undefined && achievableSpeed !== null && Number.isFinite(achievableSpeed) && achievableSpeed < speed * 0.85;
  const subjectHistory = subject?.history?.slice(-3).reverse().map(prettyHistory) ?? [];
  const subjectDestination = subject?.destination_name ?? subject?.destination;
  const trainCounts = subject?.kind.toLowerCase() === 'train'
    ? [
      subject.passengers === undefined && subject.capacity === undefined ? null : `Passengers ${subject.passengers ?? 0}${subject.capacity === undefined ? '' : ` of ${subject.capacity}`}`,
      subject.station_waiting === undefined ? null : `Station waiting ${subject.station_waiting}`,
      subject.gateway_waiting === undefined ? null : `Gateway waiting ${subject.gateway_waiting}`,
    ].filter((value): value is string => value !== null)
    : [];
  const busDetails = subject?.kind.toLowerCase() === 'bus'
    ? [
      subject.line ?? null,
      subject.nextStop ? `Next stop · ${subject.nextStop}` : null,
      subject.passengers === undefined && subject.capacity === undefined ? null : `Load ${subject.passengers ?? 0}${subject.capacity === undefined ? '' : ` of ${subject.capacity}`}`,
      subject.dwelling === undefined ? null : subject.dwelling ? 'Dwelling at stop' : 'In motion',
      subject.waiting === undefined ? null : `Waiting ${subject.waiting}`,
    ].filter((value): value is string => value !== null)
    : [];
  const busToFollow = activeBusToFollow(world);
  const renewalProject = world?.projects.find(project => project.kind === 'redevelopment' && project.progress < 1);
  const renewalSite = renewalProject && world?.buildings.find(building => building.parcel_id === renewalProject.parcel_id);
  const graphicsError = Boolean(error && /graphics|webgl|renderer/i.test(error) && !/could not restore the saved city/i.test(error));
  const forceWebGL = typeof window !== 'undefined' && new URLSearchParams(window.location.search).has('webgl');

  return (
    <div className="city-ui" data-contrast={contrast} data-masthead-tone={mastheadTone} aria-label="City controls">
      <header className="city-masthead">
        <div className="city-brand" aria-label="Tokyo, Last Train, First Light">
          <div className="city-brand__eyebrow"><span className="city-brand__jp">東京</span><span className="city-brand__rule" />A LIVING CITY</div>
          <div className="city-brand__title">Last Train,<br />First Light</div>
        </div>
        <div className="city-clock" aria-live="polite">
          <div className="city-clock__top"><span className="city-clock__dot" />{preview === null ? 'LIVE CITY CLOCK' : 'LIGHT PREVIEW'}</div>
          <div className="city-clock__time">{shownTime}</div>
          {(paused || simulationOverloaded) && <div className="city-clock__status" role="status" aria-live="polite"><span />{paused ? 'SIMULATION PAUSED' : `RUNNING ${achievableSpeed!.toFixed(1)}× · REQUESTED ${speed}×`}</div>}
          <div className="city-clock__meta">{clockLabel}<span className="city-clock__divider">/</span>{world ? stageTitle(world.stage) : ready ? 'Preparing' : 'Opening the city'}</div>
        </div>
      </header>

      {(error || message || !ready) && (
        <div className={`city-status${error ? ' city-status--error' : ''}`} role={error ? 'alert' : 'status'}>
          <span className="city-status__mark">{error ? '!' : <span />}</span>
          <span>{error || message || 'The first streets are taking shape…'}</span>
          {error && graphicsError && <span className="city-status__actions"><button type="button" onClick={onGraphicsRetry}>Retry</button><button type="button" onClick={onGraphicsFallback}>{forceWebGL ? 'Try default renderer' : 'Use WebGL fallback'}</button></span>}
          {error && /save|quota|storage/i.test(error) && onExport && <button type="button" onClick={onExport}>Export this city</button>}
          {error && onDismissError && <button type="button" className="city-status__close" aria-label="Dismiss error message" onClick={onDismissError}><X size={14} /></button>}
        </div>
      )}

      {preview !== null && (
        <div className="light-preview" aria-label="Lighting preview controls">
          <div className="light-preview__heading"><span>LIGHTING PREVIEW</span><button type="button" onClick={() => onPreview(null)}>Return to live <RotateCcw size={13} /></button></div>
          <label className="visually-hidden" htmlFor="lighting-hour">Preview hour</label>
          <input id="lighting-hour" type="range" min="0" max="24" step="1" value={preview} onChange={(event) => onPreview(Number(event.target.value))} />
          <div className="light-preview__ends"><span>00:00</span><strong>{String(preview).padStart(2, '0')}:00</strong><span>24:00</span></div>
        </div>
      )}

        <div className={`scene-caption${panel || preview !== null ? ' is-obscured' : ''}`} aria-live="polite">
        <span className="scene-caption__line" />
        <span className="scene-caption__label">NOW OBSERVING</span>
        <span className="scene-caption__scene">{sceneLabel || 'A neighborhood, finding its rhythm'}</span>
      </div>

      {selected === null && subject && (
        <aside className="subject-inspector" aria-label="Following subject">
          <div className="subject-inspector__copy"><span className="panel-kicker">SUBJECT · {prettyKind(subject.kind)}</span><strong>{subject.name}</strong><span>{prettyKind(subject.activity)}{subjectDestination ? ` · ${subjectDestination}` : ''}</span>{trainCounts.map((count) => <span key={count}>{count}</span>)}{busDetails.map((detail) => <span key={detail}>{detail}</span>)}{subjectHistory.map((line, index) => <span key={`${index}-${line}`}>{line}</span>)}</div>
          <button type="button" className="subject-inspector__follow" onClick={() => onFollow(subject.id)}>Follow this {subject.kind.toLowerCase()}</button>
          <button type="button" className="subject-inspector__close" aria-label="Stop following" title="Stop following" onClick={onCloseInspector}><X size={15} /></button>
        </aside>
      )}

      <nav className="control-dock" aria-label="Simulation controls">
        {preparingStyle && <span className="style-preparing" role="status" aria-live="polite" aria-atomic="true">Preparing {preparingStyle === 'miniature' ? 'Miniature' : 'Anime'}…</span>}
        <div className="control-dock__modes" role="group" aria-label="Camera mode">
          {modeOptions.map(({ value, label, icon: Icon, hint }) => (
            <button key={value} type="button" className={`dock-mode${mode === value ? ' is-active' : ''}`} aria-pressed={mode === value} title={hint} onClick={() => onMode(value)}>
              <Icon size={17} strokeWidth={1.7} /><span>{label}</span>
            </button>
          ))}
        </div>
        <span className="dock-separator" aria-hidden="true" />
        <button type="button" className="dock-play" aria-label={paused ? 'Resume simulation' : 'Pause simulation'} title={paused ? 'Resume simulation' : 'Pause simulation'} onClick={onPause}>
          {paused ? <Play size={17} fill="currentColor" /> : <Pause size={17} fill="currentColor" />}
        </button>
        <label className="speed-control"><span className="visually-hidden">Simulation speed</span><select aria-label="Simulation speed" value={speed} onChange={(event) => onSpeed(Number(event.target.value))}>
          {[1, 2, 4, 8].map((n) => <option key={n} value={n}>{n}×</option>)}
        </select><ChevronDown size={12} aria-hidden="true" /></label>
        <span className="dock-separator dock-separator--short" aria-hidden="true" />
        <div className="dock-segment dock-desktop-visual" role="group" aria-label="Visual style">
          <button type="button" className={`dock-pill${style === 'anime' ? ' is-active' : ''}`} aria-pressed={style === 'anime'} onClick={() => onStyle('anime')} title="Anime rendering">Anime</button>
          <button type="button" className={`dock-pill${style === 'miniature' ? ' is-active' : ''}`} aria-pressed={style === 'miniature'} onClick={() => onStyle('miniature')} title="Miniature rendering">Miniature</button>
        </div>
        <button type="button" className={`dock-icon dock-desktop-focus${focus ? ' is-active' : ''}`} aria-label={`${focus ? 'Disable' : 'Enable'} depth of field`} aria-pressed={focus} title="Depth of field" onClick={onFocus}><Focus size={17} strokeWidth={1.7} /></button>
        <label className="dock-light dock-desktop-light" title="Preview lighting by hour">
          <Clock3 size={16} strokeWidth={1.7} /><span className="visually-hidden">Preview lighting by hour</span>
          <input type="range" aria-label="Preview lighting by hour" min="0" max="24" step="1" value={preview ?? (world ? Math.floor(world.minute_of_day / 60) : 12)} onChange={(event) => onPreview(Number(event.target.value))} />
        </label>
        <div className="dock-spacer" />
        <button type="button" className={`dock-icon dock-mobile-visual${panel === 'visual' ? ' is-active' : ''}`} aria-label="Visual settings" aria-expanded={panel === 'visual'} title="Visual settings" onClick={(event) => togglePanel('visual', event)}><SlidersHorizontal size={17} strokeWidth={1.7} /></button>
        <button type="button" className={`dock-icon${sound ? '' : ' is-muted'}`} aria-label={sound ? 'Sound settings' : 'Enable sound and open settings'} aria-expanded={panel === 'sound'} title="Sound settings" onClick={(event) => togglePanel('sound', event)}>
          {sound ? <Volume2 size={17} strokeWidth={1.7} /> : <VolumeX size={17} strokeWidth={1.7} />}
        </button>
        <button type="button" className={`dock-icon${panel === 'world' ? ' is-active' : ''}`} aria-label="World and saves" aria-expanded={panel === 'world'} title="World and saves" onClick={(event) => togglePanel('world', event)}><Bookmark size={17} strokeWidth={1.7} /></button>
        <button type="button" className={`dock-icon dock-details${panel === 'details' ? ' is-active' : ''}`} aria-label="City details" aria-expanded={panel === 'details'} title="City details" onClick={(event) => togglePanel('details', event)}><CircleHelp size={17} strokeWidth={1.7} /></button>
      </nav>

      {panel === 'visual' && (
        <section className="floating-panel visual-panel" aria-label="Visual settings">
          <div className="floating-panel__head"><div><span className="panel-kicker">LOOK & LIGHT</span><h2 ref={panelHeading} tabIndex={-1}>Visual settings</h2></div><button type="button" className="panel-close" aria-label="Close visual settings" onClick={closePanel}><X size={16} /></button></div>
          <div className="mobile-visual-control"><span className="panel-kicker">RENDERING STYLE</span><div className="dock-segment" role="group" aria-label="Visual style">
            <button type="button" className={`dock-pill${style === 'anime' ? ' is-active' : ''}`} aria-pressed={style === 'anime'} onClick={() => onStyle('anime')}>Anime</button>
            <button type="button" className={`dock-pill${style === 'miniature' ? ' is-active' : ''}`} aria-pressed={style === 'miniature'} onClick={() => onStyle('miniature')}>Miniature</button>
          </div></div>
          <button type="button" className={`visual-focus-toggle${focus ? ' is-active' : ''}`} aria-pressed={focus} onClick={onFocus}><Focus size={17} />Depth of field<span>{focus ? 'On' : 'Off'}</span></button>
          <label className="visual-light-control"><span className="panel-kicker">LIGHTING PREVIEW</span><input type="range" aria-label="Preview lighting by hour" min="0" max="24" step="1" value={preview ?? (world ? Math.floor(world.minute_of_day / 60) : 12)} onChange={(event) => onPreview(Number(event.target.value))} /><span className="light-preview__ends"><span>00:00</span><strong>{preview === null ? 'LIVE' : hourTime(preview)}</strong><span>24:00</span></span></label>
          {preview !== null && <button type="button" className="visual-return-live" onClick={() => onPreview(null)}><RotateCcw size={15} />Return to live lighting</button>}
        </section>
      )}

      {panel === 'sound' && (
        <section className="floating-panel sound-panel" aria-label="Sound settings">
          <div className="floating-panel__head"><div><span className="panel-kicker">LISTENING</span><h2 ref={panelHeading} tabIndex={-1}>City sound</h2></div><button type="button" className="panel-close" aria-label="Close sound settings" onClick={closePanel}><X size={16} /></button></div>
          <button className={`sound-toggle${sound ? ' is-on' : ''}`} onClick={onSound} aria-pressed={sound}><span className="sound-toggle__icon">{sound ? <AudioLines size={16} /> : <VolumeX size={16} />}</span><span>{sound ? 'Sound on' : 'Sound off'}</span><span className="switch-track"><i /></span></button>
          <label className="level-row"><span><Headphones size={14} />Music</span><input type="range" min="0" max="1" step="0.01" value={musicLevel} onChange={(event) => onLevels(Number(event.target.value), cityLevel)} /><output>{Math.round(musicLevel * 100)}</output></label>
          <label className="level-row"><span><Volume2 size={14} />City</span><input type="range" min="0" max="1" step="0.01" value={cityLevel} onChange={(event) => onLevels(musicLevel, Number(event.target.value))} /><output>{Math.round(cityLevel * 100)}</output></label>
          <p className="panel-note">A little room for the river, trains, and evening streets.</p>
        </section>
      )}

      {panel === 'world' && (
        <section className="floating-panel world-panel" aria-label="World and saves">
          <div className="floating-panel__head"><div><span className="panel-kicker">WORLD NO. {world?.seed ?? '—'}</span><h2 ref={panelHeading} tabIndex={-1}>Keep this place</h2></div><button type="button" className="panel-close" aria-label="Close world settings" onClick={closePanel}><X size={16} /></button></div>
          <div className="save-actions"><button onClick={onSave}><Bookmark size={15} />Save this city</button><button onClick={onResume}><RotateCcw size={15} />Resume saved city</button></div>
          {(onExport || onImport) && <div className="portable-actions">
            {onExport && <button type="button" className="text-button" onClick={onExport}>Export this city</button>}
            {onImport && <>
              <button type="button" className="text-button" onClick={() => importInput.current?.click()}>Import</button>
              <input ref={importInput} type="file" accept="application/json,.json" aria-label="Import city checkpoint" hidden onChange={(event) => {
                const file = event.currentTarget.files?.[0];
                event.currentTarget.value = '';
                if (file) onImport(file);
              }} />
            </>}
          </div>}
          <button type="button" className="world-notes-link" onClick={() => { focusPanelOnOpen.current = true; setPanel('details'); }}><CircleHelp size={14} />Open city field notes</button>
          <div className="growth-setting"><label htmlFor="city-growth">City growth</label><select id="city-growth" aria-label="City growth pace" value={development} onChange={(event) => onDevelopment(Number(event.target.value))}>{[1, 2, 4, 8, 16].map((pace) => <option key={pace} value={pace}>{pace}×</option>)}</select><p>Construction and long-term change</p></div>
          <div className="new-world">
            <label htmlFor="new-city-seed">Begin another city</label>
            <div className="new-world__form"><input id="new-city-seed" inputMode="numeric" value={seedText} placeholder="Enter a seed" onChange={(event) => { setSeedText(event.target.value); setConfirmSeed(null); }} onKeyDown={(event) => { if (event.key === 'Enter') submitNewWorld(); }} /><button onClick={submitNewWorld} disabled={!/^\d+$/.test(seedText.trim())}>Continue</button></div>
            {confirmSeed !== null && <div className="new-world__confirm" role="group" aria-label="Confirm new city"><span>Start city {confirmSeed}? {world ? 'The current place will be saved first.' : 'This begins a fresh city.'}</span><button onClick={startNewWorld}>Start city</button><button className="text-button" onClick={() => setConfirmSeed(null)}>Keep watching</button></div>}
          </div>
        </section>
      )}

      {panel === 'details' && (
        <section className="floating-panel details-panel" aria-label="City details">
          <div className="floating-panel__head"><div><span className="panel-kicker">FIELD NOTES · DAY {world?.day ?? '—'}</span><h2 ref={panelHeading} tabIndex={-1}>{selected?.name || 'A city in the making'}</h2></div><button type="button" className="panel-close" aria-label="Close city details" onClick={() => { closePanel(); if (selected) onCloseInspector(); }}><X size={16} /></button></div>
          {selected ? <>
            <p className="place-subtitle">{buildingSubtitle(selected)}</p>
            <div className="place-history"><span className="panel-kicker">PLACE HISTORY</span><p>{selected.history.length ? selected.history.slice(-3).join(' · ') : `Standing since day ${selected.built_day}.`}</p></div>
            <button className="follow-button" onClick={() => onFollow(selected.id)}><Eye size={15} />Stay with this place</button>
          </> : <>
            <p className="place-subtitle">Small changes add up along these streets.</p>
            <div className="city-facts"><span><b>{world?.stats.buildings ?? 0}</b> buildings</span><span><b>{world?.stats.active_travelers ?? 0}</b> journeys underway</span><span><b>{world?.stats.active_projects ?? 0}</b> works in progress</span></div>
          </>}
          <ContrastToggle contrast={contrast} onContrast={onContrast} />
          <LensControl lens={lens} onLens={onLens} />
          <p className="lens-note">Focal length changes framing. Focus remains unchanged.</p>
          <RecentEvents events={events} />
          {world?.trains[0] && <button className="follow-button train-follow" onClick={() => onFollow(world.trains[0].id)}><Eye size={15} />Follow a train</button>}
          {busToFollow && <button className="follow-button train-follow" onClick={() => onFollow(busToFollow.id)}><Eye size={15} />Follow a bus</button>}
          {renewalSite && <button className="follow-button train-follow" onClick={() => onFollow(renewalSite.id)}><Eye size={15} />Visit the renewal site</button>}
          {residents.length > 0 && <div className="resident-list"><span className="panel-kicker">RESIDENTS IN MOTION</span><ul>{residents.map((resident) => <li key={resident.id}><span><b>{resident.name}</b><small>{prettyKind(resident.activity)}</small></span><button type="button" onClick={() => onFollow(resident.id)} aria-label={`Follow ${resident.name}`}><Eye size={14} /></button></li>)}</ul></div>}
          {(metrics || (quality && onQuality) || (import.meta.env.DEV && (onReviewMature || onReviewSkyline))) && <details className="render-note"><summary>Renderer details</summary>{import.meta.env.DEV && <><ReviewRecording/><ReviewCameraPose read={props.onReadCameraPose} apply={props.onApplyCameraPose} fixedQuality={props.fixedReviewQuality} onFixedQuality={props.onFixedReviewQuality}/><label className="quality-control">Review building ID<input aria-label="Review building ID" inputMode="numeric" value={reviewBuildingId} onChange={event=>setReviewBuildingId(event.target.value)}/></label><button type="button" className="render-note__review" disabled={!reviewBuilding||!props.onReviewBuilding} onClick={()=>{if(reviewBuilding)props.onReviewBuilding?.(reviewBuilding.id);}}>Frame review building</button></>}{import.meta.env.DEV && props.onReviewStreetLights && <button type="button" className="render-note__review" onClick={()=>setStreetLightsOn(props.onReviewStreetLights!())}>Nearby light contribution: {streetLightsOn?'on':'off'}</button>}{import.meta.env.DEV && props.onReviewWaterReflection && <button type="button" className="render-note__review" onClick={()=>setRawWaterReflection(props.onReviewWaterReflection!())}>Water view: {rawWaterReflection?'raw reflection':'normal'}</button>}{metrics && <><RenderTimingNote metrics={metrics}/><p>{metrics.calls} draws · {metrics.triangles.toLocaleString()} triangles · {metrics.textures} textures · {metrics.geometries} shapes</p>{metrics.pendingBuildings !== undefined && <p>{metrics.pendingBuildings} buildings queued · {metrics.pendingChunks ?? 0} neighborhood batches queued · {metrics.pendingSidewalks ?? 0} sidewalk rebuilds queued · {metrics.pendingRoads ?? 0} road rebuilds queued</p>}{import.meta.env.DEV && metrics.styleSwitch && <p>Last style switch to {metrics.styleSwitch.target}: {metrics.styleSwitch.compiledSamples}/{metrics.styleSwitch.samples} shader samples · prepare {metrics.styleSwitch.prepareMs.toFixed(0)} ms · submit {metrics.styleSwitch.submitMs.toFixed(0)} ms (largest {metrics.styleSwitch.maxSubmitMs.toFixed(0)} ms) · await {metrics.styleSwitch.waitMs.toFixed(0)} ms · apply {metrics.styleSwitch.applyMs.toFixed(0)} ms · {metrics.styleSwitch.outcome}</p>}</>}{quality && onQuality && <label className="quality-control">Render quality<select aria-label="Render quality" value={quality} onChange={(event) => onQuality(event.target.value as 'high' | 'balanced')}><option value="high">High detail</option><option value="balanced">Balanced</option></select></label>}{import.meta.env.DEV && onReviewMature && <button type="button" className="render-note__review" onClick={onReviewMature}>Review mature city</button>}{import.meta.env.DEV && onReviewSkyline && <button type="button" className="render-note__review" onClick={onReviewSkyline}>View skyline</button>}{import.meta.env.DEV && onReviewStation && <button type="button" className="render-note__review" onClick={onReviewStation}>View station</button>}{import.meta.env.DEV && onReviewWaterfront && <><button type="button" className="render-note__review" onClick={()=>onReviewWaterfront('overhead')}>Hold waterfront view</button><button type="button" className="render-note__review" onClick={()=>onReviewWaterfront('low')}>Hold low waterfront view</button></>}</details>}
        </section>
      )}
    </div>
  );
}

export function RenderTimingNote({metrics}:{metrics:NonNullable<ControlsProps['metrics']>}){return <><p>{metrics.timing?.samples===0?'Collecting frame timings':`${Math.round(metrics.fps)} fps`} · <span className="render-note__cpu">CPU {metrics.frameMs.toFixed(1)} ms</span> · {metrics.backend}</p>{metrics.timing && <p>{metrics.timing.phase} · {metrics.timing.samples} frames · frame p95 {metrics.timing.frameP95Ms.toFixed(1)} ms · CPU p95 {metrics.timing.cpuP95Ms.toFixed(1)} ms · longest frame {metrics.timing.maxFrameMs.toFixed(1)} ms</p>}</>;}
