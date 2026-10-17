/** Import boundaries between web layers. Run with `npm run lint:deps`. */
const forbid = (name, from, to, comment) => ({
  name,
  comment,
  severity: 'error',
  from: { path: from },
  to: { path: to },
});

module.exports = {
  forbidden: [
    forbid(
      'render-is-presentation-only',
      '^web/src/render/',
      '^web/src/(worker|storage|ui|camera|audio|app|dev)/',
      'The renderer reads snapshots; camera and audio are wired in by the app composition root.',
    ),
    forbid(
      'ui-does-not-reach-into-render',
      '^web/src/ui/',
      '^web/src/(render|worker|camera|audio)/',
      'UI talks to the app through props and callbacks.',
    ),
    forbid(
      'worker-owns-the-engine-only',
      '^web/src/worker/',
      '^web/src/(render|ui|camera|audio|storage|app|dev)/',
      'The worker owns simulation state and never depends on presentation.',
    ),
    forbid('optics-is-pure', '^web/src/optics/', '^web/src/(?!optics/)', 'Optics math has no app dependencies.'),
    forbid('audio-is-independent', '^web/src/audio/', '^web/src/(render|ui|camera|worker|app)/', ''),
    forbid('production-never-imports-dev', '^web/src/(?!dev/|main\\.tsx)', '^web/src/dev/', ''),
    {
      name: 'no-circular-across-layers',
      severity: 'warn',
      from: { path: '^web/src/' },
      to: { circular: true, pathNot: '^web/src/render/' },
    },
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    exclude: { path: '^(public|dist|node_modules)/' },
    tsConfig: { fileName: 'tsconfig.json' },
    tsPreCompilationDeps: true,
  },
};
