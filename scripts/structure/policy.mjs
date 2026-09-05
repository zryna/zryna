export const POLICY_PATH = 'scripts/repository-structure-policy.json';
export const INITIAL_COMMIT = '885bb4d863ad112566add72fab6d2931587b71b1';
export const SOURCE_EXTENSIONS = ['.rs', '.mjs', '.js', '.cjs', '.ts', '.sh', '.ps1', '.py'];

export function fail(message) { throw new Error(`structure: ${message}`); }

export function physicalLines(text) {
  if (text.length === 0) return 0;
  return (text.match(/\n/g)?.length ?? 0) + (text.endsWith('\n') ? 0 : 1);
}

export function portablePath(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9_.\/-]+$/.test(value)
      || value.startsWith('/') || value.split('/').some(part => !part || part === '.' || part === '..'
        || /[. ]$/.test(part) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(part))) {
    fail(`invalid exact path: ${JSON.stringify(value)}`);
  }
  return value;
}

function keys(record, expected, label) {
  if (!record || typeof record !== 'object' || Array.isArray(record)
      || Object.keys(record).sort().join() !== [...expected].sort().join()) fail(`invalid ${label} fields`);
}

function text(value, label) {
  if (typeof value !== 'string' || value.trim().length < 3) fail(`missing ${label}`);
}

export function validatePolicy(policy, today) {
  keys(policy, ['version', 'anchor', 'baseline', 'classifications', 'exceptions'], 'policy');
  if (policy.version !== 1 || !/^[a-f0-9]{40}$/.test(policy.anchor)) fail('invalid version or anchor commit');
  for (const group of ['baseline', 'classifications', 'exceptions']) {
    if (!Array.isArray(policy[group])) fail(`${group} must be an array`);
    const seen = new Set();
    for (const entry of policy[group]) {
      const expected = group === 'baseline' ? ['path', 'lines', 'production'] : group === 'classifications'
        ? ['path', 'kind', 'owner', 'reason', 'review']
        : ['path', 'owner', 'reason', 'ceiling', 'review', 'expires'];
      keys(entry, expected, group);
      portablePath(entry.path);
      if (seen.has(entry.path.toLowerCase())) fail(`duplicate ${group} path: ${entry.path}`);
      seen.add(entry.path.toLowerCase());
      if (group === 'baseline') {
        if (!Number.isSafeInteger(entry.lines) || entry.lines <= 500) fail(`invalid baseline count: ${entry.path}`);
        if (typeof entry.production !== 'boolean') fail(`invalid initial production classification: ${entry.path}`);
      } else {
        for (const field of ['owner', 'reason', 'review']) text(entry[field], `${group} ${field}`);
        if (group === 'classifications' && !['test', 'fixture', 'generated'].includes(entry.kind)) fail(`invalid classification: ${entry.path}`);
        if (group === 'exceptions') {
          if (!Number.isSafeInteger(entry.ceiling) || entry.ceiling <= 500) fail(`invalid exception ceiling: ${entry.path}`);
          if (!/^\d{4}-\d{2}-\d{2}$/.test(entry.expires)
              || !Number.isFinite(Date.parse(`${entry.expires}T00:00:00Z`))
              || new Date(`${entry.expires}T00:00:00Z`).toISOString().slice(0, 10) !== entry.expires) fail(`invalid UTC expiry: ${entry.path}`);
          if (entry.expires <= today) fail(`expired exception: ${entry.path} (expires at start of ${entry.expires} UTC)`);
        }
      }
    }
  }
  return policy;
}

export function reviewChanges(previous, current) {
  const messages = [];
  for (const group of ['baseline', 'classifications', 'exceptions']) {
    const before = new Map((previous?.[group] ?? []).map(entry => [entry.path, entry]));
    const after = new Map(current[group].map(entry => [entry.path, entry]));
    for (const path of [...new Set([...before.keys(), ...after.keys()])].sort()) {
      if (JSON.stringify(before.get(path)) === JSON.stringify(after.get(path))) continue;
      messages.push(`REVIEW ${group} ${path}: ${JSON.stringify(before.get(path) ?? null)} -> ${JSON.stringify(after.get(path) ?? null)}`);
    }
  }
  return messages;
}
