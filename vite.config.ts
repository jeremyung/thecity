import { defineConfig } from 'vitest/config';
export default defineConfig({
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
  worker: { format: 'es' },
  // Geometry and WASM suites are CPU/memory heavy. Bound concurrency and
  // collect only this checkout's tests, never local tool or agent checkouts.
  test: { include: ['tests/**/*.test.ts'], maxWorkers: 2 },
});
