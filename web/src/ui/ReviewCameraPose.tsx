import { useState } from 'react';

export type ReviewPosePoint = [number, number, number];
export type ApplyReviewPose = (position: ReviewPosePoint, target: ReviewPosePoint) => boolean;

/** Development-only capture and exact safe framing for matched visual reviews. */
export default function ReviewCameraPose({
  read,
  apply,
  fixedQuality,
  onFixedQuality,
}: {
  read?: () => string;
  apply?: ApplyReviewPose;
  fixedQuality?: boolean;
  onFixedQuality?: (enabled: boolean) => void;
}) {
  const [capture, setCapture] = useState('');
  const [request, setRequest] = useState('');
  const [message, setMessage] = useState('');
  const applyPose = () => {
    try {
      const pose = JSON.parse(request);
      const point = (value: unknown): value is ReviewPosePoint =>
        Array.isArray(value) &&
        value.length === 3 &&
        value.every((item) => typeof item === 'number' && Number.isFinite(item));
      if (!point(pose?.position) || !point(pose?.target))
        throw new Error('Expected position and target arrays with three finite numbers each.');
      if (!apply?.(pose.position, pose.target)) {
        setMessage('Pose rejected: outside camera limits or inside an obstacle.');
        return;
      }
      setCapture(read?.() ?? '');
      setMessage('Exact pose applied.');
    } catch {
      setMessage('Expected JSON with position and target arrays of three finite numbers.');
    }
  };
  return (
    <section aria-label="Camera review capture">
      {onFixedQuality && (
        <label>
          <input
            type="checkbox"
            checked={fixedQuality ?? false}
            onChange={(event) => onFixedQuality(event.target.checked)}
          />
          Lock review quality
        </label>
      )}
      {fixedQuality && (
        <p>Fixed: 1× pixels, 2048 shadow map, 35% water reflection. Adaptive quality paused.</p>
      )}
      <button type="button" disabled={!read} onClick={() => setCapture(read?.() ?? '')}>
        Read camera pose
      </button>
      <label>
        Exact review pose
        <textarea
          aria-label="Exact review pose"
          value={request}
          onChange={(event) => setRequest(event.target.value)}
        />
      </label>
      <button type="button" disabled={!apply} onClick={applyPose}>
        Apply exact review pose
      </button>
      {message && <p role="status">{message}</p>}
      {capture && (
        <pre
          aria-label="Captured camera pose"
          style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}
        >
          {capture}
        </pre>
      )}
    </section>
  );
}
