import { defineConfig } from 'vitest/config';
export default defineConfig({
  server: { port: 5173, strictPort: true },
  build: { target: 'es2022' },
  worker: { format: 'es' },
  // Agent worktrees under .claude are full checkouts; never run their tests here.
  test: { exclude: ['**/node_modules/**', '**/dist/**', '.claude/**'] },
});
