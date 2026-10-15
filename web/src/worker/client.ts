import { decodeMovementPacket, type MovementSupport } from '../movement-support';
import { WORKER_PROTOCOL_VERSION, type Metrics, type WorkerInput, type WorkerOutput, type World } from "../types";

type WorldListener = (world: World) => void;
type MovementListener = (movement: Float32Array, tick: number, minute: number, day: number, trainDoors: Float32Array, support: ReadonlyMap<number, MovementSupport>) => void;
type MetricsListener = (metrics: Metrics) => void;
type ErrorListener = (error: Error) => void;
type PendingRequest = {
  epoch: number;
  type: "save" | "inspect";
  resolve: (value: string | unknown) => void;
  reject: (error: Error) => void;
  timeout: number;
};

const REQUEST_TIMEOUT_MS = 15_000;

/** Main-thread protocol client. Movement subscribers must copy data they retain. */
export class SimulationClient {
  private readonly worker: Worker;
  private epoch = 0;
  get worldEpoch(){return this.epoch;}
  private requestId = 0;
  private disposed = false;
  private protocolFailed = false;
  private lastRevision = -1;
  private lastPresentationRevision: string | undefined;
  private resyncPending = false;
  private pending = new Map<number, PendingRequest>();
  private nextControlSequence = 0;
  private lastAppliedControlSequence = 0;
  private controlAckTimeout: ReturnType<typeof setTimeout> | null = null;
  private worldListeners = new Set<WorldListener>();
  private movementListeners = new Set<MovementListener>();
  private metricsListeners = new Set<MetricsListener>();
  private errorListeners = new Set<ErrorListener>();
  private queuedControls: Omit<Extract<WorkerInput, { type: "control" }>, "type" | "protocolVersion" | "epoch" | "sequence"> = {};
  private restoreEpoch: number | null = null;
  private restoreDevelopmentOverride = false;

  constructor() {
    this.worker = new Worker(new URL("./simulation.worker.ts", import.meta.url), { type: "module" });
    this.worker.onmessage = (event: MessageEvent<WorkerOutput>) => this.receive(event.data);
    this.worker.onerror = (event) => {
      event.preventDefault();
      this.clearControlAckTimeout();
      const error = new Error(event.message || "Simulation worker failed");
      this.emitError(error);
      this.failAll(error);
    };
    this.worker.onmessageerror = () => {
      this.clearControlAckTimeout();
      const error = new Error("Could not read a message from the simulation worker");
      this.emitError(error);
      this.failAll(error);
    };
  }

  init(seed: number, save?: string): void {
    this.ensureActive();
    this.clearControlAckTimeout();
    this.epoch++;
    this.nextControlSequence = 0;
    this.lastAppliedControlSequence = 0;
    this.lastRevision = -1;
    this.lastPresentationRevision = undefined;
    this.resyncPending = false;
    if (save !== undefined) {
      // Saved development pace is authoritative on restore. Other viewer
      // preferences (pause, hidden, playback speed) remain session settings.
      delete this.queuedControls.development;
      this.restoreEpoch = this.epoch;
      this.restoreDevelopmentOverride = false;
    } else {
      this.restoreEpoch = null;
      this.restoreDevelopmentOverride = false;
    }
    this.failAll(new Error("World changed before the request completed"));
    const message: WorkerInput = save === undefined
      ? { type: "init", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: this.epoch, seed: seed >>> 0 }
      : { type: "init", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: this.epoch, seed: seed >>> 0, save };
    try {
      this.worker.postMessage(message);
    } catch (error) {
      this.emitError(error instanceof Error ? error : new Error(String(error)));
      return;
    }
    this.postControl();
  }

  controls(values: { paused?: boolean; hidden?: boolean; speed?: number; development?: number }): void {
    this.ensureActive();
    if (values.paused !== undefined) this.queuedControls.paused = values.paused;
    if (values.hidden !== undefined) this.queuedControls.hidden = values.hidden;
    if (values.speed !== undefined) this.queuedControls.speed = values.speed;
    if (values.development !== undefined) {
      this.queuedControls.development = values.development;
      if (this.restoreEpoch === this.epoch) this.restoreDevelopmentOverride = true;
    }
    this.postControl();
  }

  subscribeWorld(listener: WorldListener): () => void {
    this.worldListeners.add(listener);
    return () => this.worldListeners.delete(listener);
  }

  subscribeMovement(listener: MovementListener): () => void {
    this.movementListeners.add(listener);
    return () => this.movementListeners.delete(listener);
  }

  subscribeMetrics(listener: MetricsListener): () => void {
    this.metricsListeners.add(listener);
    return () => this.metricsListeners.delete(listener);
  }

  onError(listener: ErrorListener): () => void {
    this.errorListeners.add(listener);
    return () => this.errorListeners.delete(listener);
  }

  save(): Promise<string> {
    return this.request<string>("save");
  }

  inspect(id: number): Promise<unknown> {
    return this.request<unknown>("inspect", id);
  }

  dispose(): void {
    if (this.disposed) return;
    this.clearControlAckTimeout();
    this.disposed = true;
    this.failAll(new Error("Simulation client was disposed"));
    this.worker.terminate();
    this.worldListeners.clear();
    this.movementListeners.clear();
    this.metricsListeners.clear();
    this.errorListeners.clear();
  }

  private request<T extends string | unknown>(type: "save" | "inspect", id?: number): Promise<T> {
    this.ensureActive();
    if (this.epoch === 0) return Promise.reject(new Error("Simulation has not been initialized"));
    const request = ++this.requestId;
    return new Promise<T>((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        this.pending.delete(request);
        reject(new Error(`Simulation ${type} request timed out`));
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(request, {
        epoch: this.epoch,
        type,
        resolve: (value) => resolve(value as T),
        reject,
        timeout,
      });
      const message: WorkerInput = type === "save"
        ? { type: "save", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: this.epoch, request }
        : { type: "inspect", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: this.epoch, request, id: id ?? -1 };
      try {
        this.worker.postMessage(message);
      } catch (error) {
        this.pending.delete(request);
        window.clearTimeout(timeout);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  private postControl(): void {
    if (this.epoch === 0 || this.disposed) return;
    const sequence = ++this.nextControlSequence;
    const message: WorkerInput = {
      type: "control",
      protocolVersion: WORKER_PROTOCOL_VERSION,
      epoch: this.epoch,
      sequence,
      ...this.queuedControls,
    };
    try {
      this.worker.postMessage(message);
      this.clearControlAckTimeout();
      this.controlAckTimeout = globalThis.setTimeout(() => {
        this.controlAckTimeout = null;
        if (this.disposed || this.protocolFailed || this.epoch === 0 || sequence <= this.lastAppliedControlSequence) return;
        this.emitError(new Error(`Simulation controls were not acknowledged within ${REQUEST_TIMEOUT_MS / 1000} seconds (sequence ${sequence}). The interface has not restarted the worker or changed the requested controls.`));
      }, REQUEST_TIMEOUT_MS);
    } catch (error) {
      this.clearControlAckTimeout();
      this.emitError(error instanceof Error ? error : new Error(String(error)));
    }
  }

  private requestResync(): void {
    if (this.resyncPending || this.epoch === 0 || this.disposed) return;
    this.resyncPending = true;
    this.worker.postMessage({ type: "resync", protocolVersion: WORKER_PROTOCOL_VERSION, epoch: this.epoch });
  }

  private receive(message: WorkerOutput): void {
    if (this.disposed || this.protocolFailed) return;
    if (!message || typeof message !== "object" || message.protocolVersion !== WORKER_PROTOCOL_VERSION) {
      const received = message && typeof message === "object" ? String((message as { protocolVersion?: unknown }).protocolVersion) : "missing";
      this.failProtocol(new Error(`Simulation worker protocol mismatch: the interface requires version ${WORKER_PROTOCOL_VERSION}, but the worker returned ${received}. Reload the app so both use matching files.`));
      return;
    }
    if (message.epoch !== this.epoch) {
      if (message.type === "movement") this.recycle(message.epoch, message.buffer);
      return;
    }

    switch (message.type) {
      case "world": {
        if (message.world.schema_version !== 1 || typeof message.world.presentation_revision !== "string" || !message.world.presentation_revision) {
          this.emitError(new Error(`Unsupported world schema ${message.world.schema_version}; requesting a fresh snapshot`));
          this.requestResync();
          return;
        }
        if (message.world.revision < this.lastRevision) {
          this.requestResync();
          return;
        }
        this.resyncPending = false;
        this.lastRevision = message.world.revision;
        this.lastPresentationRevision = message.world.presentation_revision;
        if (this.restoreEpoch === message.epoch) {
          if (!this.restoreDevelopmentOverride) this.queuedControls.development = message.world.development_rate;
          this.restoreEpoch = null;
          this.restoreDevelopmentOverride = false;
        }
        this.callListeners(this.worldListeners, (listener) => listener(message.world));
        break;
      }
      case "movement": {
        try {
          if (message.presentationRevision !== this.lastPresentationRevision || !message.presentationRevision) {
            this.requestResync();
            return;
          }
          if (!Number.isSafeInteger(message.wordCount) || message.wordCount < 2 || message.wordCount * Uint32Array.BYTES_PER_ELEMENT > message.buffer.byteLength) {
            throw new Error("Movement support packet has an invalid word count");
          }
          const packet = new Uint32Array(message.buffer, 0, message.wordCount);
          const decoded = decodeMovementPacket(packet);
          this.callListeners(this.movementListeners, (listener) => listener(decoded.movement, message.tick, message.minute, message.day, message.trainDoors, decoded.support));
        } catch (error) {
          this.emitError(error instanceof Error ? error : new Error(String(error)));
          this.requestResync();
        } finally {
          this.recycle(message.epoch, message.buffer);
        }
        break;
      }
      case "metrics":
        this.callListeners(this.metricsListeners, (listener) => listener(message.metrics));
        break;
      case "controlAck":
        if (!Number.isSafeInteger(message.appliedSequence) || message.appliedSequence < 0 || message.appliedSequence > this.nextControlSequence || !Number.isFinite(message.tick) || message.tick < 0) {
          this.failProtocol(new Error("Simulation worker returned an invalid control acknowledgement. Reload the app to restore communication."));
          return;
        }
        this.lastAppliedControlSequence = Math.max(this.lastAppliedControlSequence, message.appliedSequence);
        if (this.lastAppliedControlSequence >= this.nextControlSequence) this.clearControlAckTimeout();
        break;
      case "save":
        this.resolveRequest(message.request, message.data, "save");
        break;
      case "inspect":
        this.resolveRequest(message.request, message.data, "inspect");
        break;
      case "error":
        if (message.code === "protocol-version") {
          this.failProtocol(new Error(message.message));
          return;
        }
        this.emitError(new Error(message.message));
        this.failAll(new Error(message.message), message.epoch);
        break;
    }
  }

  private recycle(epoch: number, buffer: ArrayBuffer): void {
    if (this.disposed || this.protocolFailed) return;
    this.worker.postMessage({ type: "recycle", protocolVersion: WORKER_PROTOCOL_VERSION, epoch, buffer }, [buffer]);
  }

  private resolveRequest(request: number, value: string | unknown, type: "save" | "inspect"): void {
    const pending = this.pending.get(request);
    if (!pending || pending.epoch !== this.epoch || pending.type !== type) return;
    this.pending.delete(request);
    window.clearTimeout(pending.timeout);
    pending.resolve(value);
  }

  private failAll(error: Error, epoch?: number): void {
    for (const [id, pending] of this.pending) {
      if (epoch !== undefined && pending.epoch !== epoch) continue;
      this.pending.delete(id);
      window.clearTimeout(pending.timeout);
      pending.reject(error);
    }
  }

  private emitError(error: Error): void {
    this.callListeners(this.errorListeners, (listener) => listener(error));
  }

  private clearControlAckTimeout(): void {
    if (this.controlAckTimeout === null) return;
    globalThis.clearTimeout(this.controlAckTimeout);
    this.controlAckTimeout = null;
  }

  private failProtocol(error: Error): void {
    if (this.disposed || this.protocolFailed) return;
    this.protocolFailed = true;
    this.clearControlAckTimeout();
    this.failAll(error);
    // A mismatched peer cannot safely receive even buffer-recycle messages.
    // Terminating it releases its pooled buffers and prevents further work.
    this.worker.terminate();
    this.emitError(error);
  }

  private callListeners<T>(listeners: Set<T>, call: (listener: T) => void): void {
    for (const listener of listeners) {
      try {
        call(listener);
      } catch (error) {
        const normalized = error instanceof Error ? error : new Error(String(error));
        if (listeners !== this.errorListeners) this.emitError(normalized);
      }
    }
  }

  private ensureActive(): void {
    if (this.disposed) throw new Error("Simulation client has been disposed");
    if (this.protocolFailed) throw new Error("Simulation client stopped after a worker protocol mismatch; reload the app to continue");
  }
}
