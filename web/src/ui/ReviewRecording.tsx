import { useEffect, useRef, useState } from 'react';

const RECORDING_MS = 20_000;
const MAX_BYTES = 20 * 1024 * 1024;

type RecordingState = 'idle' | 'recording' | 'complete' | 'error';

/** Dev-only visual capture for reviewing the rendered city without audio or screen capture. */
export default function ReviewRecording() {
  const [state, setState] = useState<RecordingState>('idle');
  const [remainingMs, setRemainingMs] = useState(RECORDING_MS);
  const [error, setError] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState<string | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const timerRef = useRef<number | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const bytesRef = useRef(0);
  const stoppedRef = useRef(false);
  const stopReasonRef = useRef<string | null>(null);
  const downloadUrlRef = useRef<string | null>(null);

  useEffect(
    () => () => {
      if (timerRef.current !== null) window.clearInterval(timerRef.current);
      stoppedRef.current = true;
      const recorder = recorderRef.current;
      if (recorder) {
        recorder.onstop = null;
        recorder.ondataavailable = null;
        recorder.onerror = null;
        if (recorder.state !== 'inactive') recorder.stop();
      }
      chunksRef.current = [];
      recorderRef.current = null;
      streamRef.current?.getTracks().forEach((track) => track.stop());
      if (downloadUrlRef.current) URL.revokeObjectURL(downloadUrlRef.current);
    },
    [],
  );

  const finish = (message?: string) => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    recorderRef.current = null;
    if (message) {
      chunksRef.current = [];
      setError(message);
      setState('error');
      return;
    }
    const blob = new Blob(chunksRef.current, { type: 'video/webm' });
    chunksRef.current = [];
    if (!blob.size) {
      setError('The browser did not produce a recording.');
      setState('error');
      return;
    }
    const url = URL.createObjectURL(blob);
    downloadUrlRef.current = url;
    setDownloadUrl(url);
    setState('complete');
  };

  const stopRecording = (message?: string) => {
    if (message) stopReasonRef.current = message;
    if (stoppedRef.current) return;
    stoppedRef.current = true;
    if (message) setError(message);
    const recorder = recorderRef.current;
    if (recorder && recorder.state !== 'inactive') {
      recorder.stop();
    } else {
      finish(message);
    }
  };

  const startRecording = () => {
    setError(null);
    setDownloadUrl((previous) => {
      if (previous) URL.revokeObjectURL(previous);
      downloadUrlRef.current = null;
      return null;
    });
    chunksRef.current = [];
    bytesRef.current = 0;
    stoppedRef.current = false;
    stopReasonRef.current = null;
    setRemainingMs(RECORDING_MS);

    const canvas = document.querySelector<HTMLCanvasElement>('.city-canvas canvas');
    if (!canvas) {
      setError('The city canvas is not available yet.');
      setState('error');
      return;
    }
    if (typeof canvas.captureStream !== 'function' || typeof MediaRecorder === 'undefined') {
      setError('This browser does not support canvas video recording.');
      setState('error');
      return;
    }

    try {
      const stream = canvas.captureStream(30);
      streamRef.current = stream;
      const options = MediaRecorder.isTypeSupported('video/webm;codecs=vp9')
        ? { mimeType: 'video/webm;codecs=vp9' }
        : MediaRecorder.isTypeSupported('video/webm;codecs=vp8')
          ? { mimeType: 'video/webm;codecs=vp8' }
          : { mimeType: 'video/webm' };
      const recorder = new MediaRecorder(stream, options);
      recorderRef.current = recorder;
      recorder.ondataavailable = (event) => {
        if (!event.data.size) return;
        bytesRef.current += event.data.size;
        if (bytesRef.current > MAX_BYTES) {
          stopRecording(
            `Recording stopped because it exceeded the ${MAX_BYTES / 1024 / 1024} MiB size limit.`,
          );
          return;
        }
        chunksRef.current.push(event.data);
      };
      recorder.onerror = () => stopRecording('The browser encountered an error while recording.');
      recorder.onstop = () => finish(stopReasonRef.current ?? undefined);
      recorder.start(1000);
      setState('recording');
      const startedAt = Date.now();
      timerRef.current = window.setInterval(() => {
        const next = Math.max(0, RECORDING_MS - (Date.now() - startedAt));
        setRemainingMs(next);
        if (next === 0) stopRecording();
      }, 200);
    } catch (cause) {
      finish(cause instanceof Error ? cause.message : 'Could not start the city recording.');
    }
  };

  return (
    <section
      aria-label="Visual review recording"
      style={{
        display: 'grid',
        gap: 8,
        padding: 12,
        border: '1px solid rgba(255,255,255,.14)',
        borderRadius: 8,
        color: 'inherit',
        fontSize: 13,
      }}
    >
      <strong>Visual review capture</strong>
      <span role="status" aria-live="polite">
        {state === 'recording'
          ? `Recording city canvas · ${Math.ceil(remainingMs / 1000)}s left`
          : state === 'complete'
            ? 'Recording complete.'
            : state === 'error'
              ? error
              : 'Captures the city canvas only; no screen, microphone, or audio.'}
      </span>
      {state === 'recording' ? (
        <button type="button" onClick={() => stopRecording()}>
          Stop recording
        </button>
      ) : (
        <button type="button" onClick={startRecording}>
          Record 20-second visual review
        </button>
      )}
      {error && state === 'error' && <span role="alert">{error}</span>}
      {downloadUrl && (
        <a href={downloadUrl} download="city-visual-review.webm">
          Download WebM recording
        </a>
      )}
    </section>
  );
}
