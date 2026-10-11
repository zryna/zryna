export const installedNames = Object.freeze(['pure', 'declared-error', 'found', 'missing', 'empty', 'utf8',
  'omitted', 'wrong-key', 'unsupported', 'malformed', 'duplicate', 'conflicting', 'extra-grant',
  'relative-grant', 'node-override', 'root-override', 'scalar-argument', 'other-export', 'dependency',
  'build-only', 'inside-installation', 'contains-installation', 'default-directory',
  'existing-output-preserved', 'component-bytes-exact', 'component-bytes-first-extra',
  'provider-tamper', 'runtime-tamper', 'metadata-tamper',
  'extra-installation-file', 'restored-installation']);
export const positive = new Map([
  ['pure', ['ok', 'none']], ['declared-error', ['err', 'none']], ['found', ['ok', 'present']],
  ['missing', ['err', 'missing']], ['empty', ['ok', 'present']], ['utf8', ['ok', 'present']],
  ['default-directory', ['ok', 'none']], ['restored-installation', ['ok', 'none']],
  ['component-bytes-exact', ['ok', 'none']],
]);
const common = ['--target', 'wasi-command', '--profile', 'command-h1-v1', '--export', 'main', '--json'];
export function invocationArgv(name, report, paths) {
  if (name === 'existing-output-preserved') {
    return ['run', 'pure.zry', ...common, '--project-root', report.project, '--name', 'pure'];
  }
  const entry = name === 'component-bytes-exact' ? 'exact.zry' : name === 'component-bytes-first-extra' ? 'extra.zry'
    : ['declared-error'].includes(name) ? 'error.zry'
    : ['found', 'missing', 'empty', 'utf8', 'omitted', 'wrong-key', 'unsupported', 'malformed',
      'duplicate', 'conflicting', 'relative-grant'].includes(name) ? 'lookup.zry'
      : name === 'dependency' ? 'dependency.zry' : 'pure.zry';
  const grant = { found: 'found-input', missing: 'missing-input', empty: 'empty-input', utf8: 'utf8-input',
    'wrong-key': 'wrong-key-input', unsupported: 'unsupported-input', malformed: 'malformed-input',
    duplicate: 'duplicate-input', conflicting: 'conflicting-input', 'extra-grant': 'extra-input' }[name];
  const output = paths.dirname(report.installation);
  const extras = grant ? ['--grant-file', paths.join(output, `${grant}.json`)]
    : name === 'relative-grant' ? ['--grant-file', 'relative.json']
      : name === 'node-override' ? ['--node', report.nodeExecutable]
        : name === 'root-override' ? ['--root', report.sourceDirectory]
          : name === 'scalar-argument' ? ['--arg=i32:1']
            : name === 'other-export' ? ['--export', 'other'] : [];
  const project = name === 'inside-installation' ? report.installation
    : name === 'contains-installation' ? output : name === 'default-directory' ? null : report.project;
  return [name === 'build-only' ? 'build' : 'run', entry, ...common, '--name', name,
    ...(project ? ['--project-root', project] : []), ...extras];
}
