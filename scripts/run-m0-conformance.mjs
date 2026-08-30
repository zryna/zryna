import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const WORKSPACE_ROOT = resolve(dirname(SCRIPT_PATH), '..');
const MANIFEST_PATH = resolve(WORKSPACE_ROOT, 'tests', 'm0-conformance-v1.json');
const PACKAGE_PATH = resolve(WORKSPACE_ROOT, 'package.json');
const MAX_MANIFEST_BYTES = 64 * 1024;
const CANONICAL_PACKAGE_ALIAS = 'node scripts/run-m0-conformance.mjs';
const SUPPORTED_PLATFORMS = ['linux', 'windows'];
const EXPECTED_COMMAND_IDS = [
  'cargo-fetch',
  'architecture-check',
  'rust-format',
  'rust-clippy',
  'rust-tests',
  'rustdoc',
  'adapter-check',
  'adapter-test-check',
  'adapter-tests',
  'protocol-check',
  'protocol-tests',
  'closure-registry-tests',
];
const EXPECTED_COVERAGE_IDS = [
  'architecture-scanner-manifest-and-graph',
  'source-authority',
  'diagnostic-authority',
  'frontend-handshake-and-worker',
  'syntax-protocol-verifier',
  'semantic-stop-gate',
  'universal-ir-verifier',
  'javascript-backend-boundary',
  'native-mir-verifier',
  'native-backend-boundary',
  'driver-boundary',
  'typescript-adapter',
  'protocol-wire-schema',
];
const EXPECTED_FIXTURES = [
  {
    id: 'syntax-v2-valid',
    path: 'tests/fixtures/syntax-v2-valid.json',
    phase: 'protocol',
    mode: 'pass',
    expected: 'schema-accepted',
  },
  {
    id: 'syntax-v2-missing-field',
    path: 'tests/fixtures/syntax-v2-missing-field.json',
    phase: 'protocol',
    mode: 'fail',
    expected: 'schema-rejected',
  },
  {
    id: 'syntax-v2-unknown-field',
    path: 'tests/fixtures/syntax-v2-unknown-field.json',
    phase: 'protocol',
    mode: 'fail',
    expected: 'schema-rejected',
  },
  {
    id: 'typescript-adapter-v2-request',
    path: 'tests/fixtures/typescript-adapter-v2-request.json',
    phase: 'adapter',
    mode: 'pass',
    expected: 'byte-stable-response',
  },
  {
    id: 'typescript-adapter-v2-result',
    path: 'tests/fixtures/typescript-adapter-v2-result.json',
    phase: 'syntax',
    mode: 'pass',
    expected: 'authoritative-verifier-accepted',
  },
  {
    id: 'typescript-adapter-v2-error-result',
    path: 'tests/fixtures/typescript-adapter-v2-error-result.json',
    phase: 'semantics',
    mode: 'fail',
    expected: 'semantic-input-rejected',
  },
  {
    id: 'typescript-adapter-v2-warning-result',
    path: 'tests/fixtures/typescript-adapter-v2-warning-result.json',
    phase: 'semantics',
    mode: 'pass',
    expected: 'semantic-input-accepted',
  },
];
const EXPECTED_COMMANDS_SHA256 = '1af3e3c83627475879bf91cb5071e904120bd1a853b27c842eb0f8b05d6e449d';
const EXPECTED_COVERAGE_SHA256 = 'c9e34908053bbd162e73a880ece6c2c8f05b22ab1f87e69a19c7d9d45ea1724a';
const ALLOWED_EXECUTABLES = new Set(['cargo', 'node']);
const SAFE_TOKEN = /^[\x20-\x7e]+$/;

function fail(message) {
  throw new Error(`invalid M0 conformance registry: ${message}`);
}

function exactKeys(value, expected, label) {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(`${label} must contain exactly ${wanted.join(', ')}`);
  }
}

function requireString(value, label) {
  if (typeof value !== 'string' || value.length === 0 || !SAFE_TOKEN.test(value)) {
    fail(`${label} must be non-empty printable ASCII`);
  }
}

function requireExactIds(items, expected, label) {
  const actual = items.map((item) => item.id);
  if (actual.length !== expected.length || actual.some((id, index) => id !== expected[index])) {
    fail(`${label} ids or order differ from the frozen M0 registry`);
  }
  if (new Set(actual).size !== actual.length) fail(`${label} ids must be unique`);
}

function canonicalDigest(value) {
  return createHash('sha256').update(JSON.stringify(value)).digest('hex');
}

function requireFrozenRecords(items, expectedDigest, label) {
  if (canonicalDigest(items) !== expectedDigest) {
    fail(`${label} declarations differ from the frozen M0 registry`);
  }
}

function validateWorkspacePath(value, label, expectedType) {
  requireString(value, label);
  if (isAbsolute(value) || value.includes('\\') || value.split('/').includes('..')) {
    fail(`${label} must be a portable workspace-relative path`);
  }
  const absolute = resolve(WORKSPACE_ROOT, value);
  const fromRoot = relative(WORKSPACE_ROOT, absolute);
  if (fromRoot.startsWith('..') || isAbsolute(fromRoot)) fail(`${label} escapes the workspace`);
  const stats = statSync(absolute, { throwIfNoEntry: false });
  if (!stats || (expectedType === 'directory' ? !stats.isDirectory() : !stats.isFile())) {
    fail(`${label} does not name an existing ${expectedType}`);
  }
}

function actualFixturePaths() {
  const fixtureRoot = resolve(WORKSPACE_ROOT, 'tests', 'fixtures');
  return readdirSync(fixtureRoot, { withFileTypes: true })
    .map((entry) => {
      if (!entry.isFile() || entry.isSymbolicLink()) fail(`fixture entry ${entry.name} is not a regular file`);
      return `tests/fixtures/${entry.name}`;
    })
    .sort();
}

export function validateManifestDocument(manifest, fixturePaths = actualFixturePaths()) {
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) fail('root must be an object');
  exactKeys(manifest, ['schemaVersion', 'milestone', 'commands', 'coverage', 'fixtures'], 'root');
  if (manifest.schemaVersion !== 1 || manifest.milestone !== 'M0') fail('version or milestone is unsupported');
  if (!Array.isArray(manifest.commands) || !Array.isArray(manifest.coverage) || !Array.isArray(manifest.fixtures)) {
    fail('commands, coverage, and fixtures must be arrays');
  }
  requireExactIds(manifest.commands, EXPECTED_COMMAND_IDS, 'command');
  requireExactIds(manifest.coverage, EXPECTED_COVERAGE_IDS, 'coverage');
  requireExactIds(manifest.fixtures, EXPECTED_FIXTURES.map(({ id }) => id), 'fixture');

  for (const [index, command] of manifest.commands.entries()) {
    const keys = command.environment
      ? ['id', 'executable', 'args', 'platforms', 'environment']
      : ['id', 'executable', 'args', 'platforms'];
    exactKeys(command, keys, `command #${index}`);
    requireString(command.id, `command #${index} id`);
    requireString(command.executable, `command #${index} executable`);
    if (!ALLOWED_EXECUTABLES.has(command.executable)) fail(`command #${index} executable is not allowed`);
    if (!Array.isArray(command.args) || command.args.length === 0) fail(`command #${index} args must be non-empty`);
    command.args.forEach((argument, argumentIndex) => requireString(argument, `command #${index} arg #${argumentIndex}`));
    if (JSON.stringify(command.platforms) !== JSON.stringify(SUPPORTED_PLATFORMS)) {
      fail(`command #${index} must run on Linux and Windows`);
    }
    if (command.environment) {
      exactKeys(command.environment, ['RUSTDOCFLAGS'], `command #${index} environment`);
      if (command.environment.RUSTDOCFLAGS !== '-D warnings') fail(`command #${index} has an unsupported environment override`);
    }
  }
  requireFrozenRecords(manifest.commands, EXPECTED_COMMANDS_SHA256, 'command');

  const commandIds = new Set(manifest.commands.map((command) => command.id));
  for (const [index, coverage] of manifest.coverage.entries()) {
    exactKeys(coverage, ['id', 'owner', 'commandId', 'proofs'], `coverage #${index}`);
    requireString(coverage.id, `coverage #${index} id`);
    validateWorkspacePath(coverage.owner, `coverage #${index} owner`, 'directory');
    requireString(coverage.commandId, `coverage #${index} commandId`);
    if (!commandIds.has(coverage.commandId)) fail(`coverage #${index} references an unknown command`);
    if (!Array.isArray(coverage.proofs) || coverage.proofs.length === 0) fail(`coverage #${index} proofs must be non-empty`);
    coverage.proofs.forEach((proof, proofIndex) => requireString(proof, `coverage #${index} proof #${proofIndex}`));
  }
  requireFrozenRecords(manifest.coverage, EXPECTED_COVERAGE_SHA256, 'coverage');

  for (const [index, fixture] of manifest.fixtures.entries()) {
    exactKeys(fixture, ['id', 'path', 'phase', 'mode', 'expected'], `fixture #${index}`);
    for (const key of ['id', 'path', 'phase', 'mode', 'expected']) requireString(fixture[key], `fixture #${index} ${key}`);
    validateWorkspacePath(fixture.path, `fixture #${index} path`, 'file');
    if (JSON.stringify(fixture) !== JSON.stringify(EXPECTED_FIXTURES[index])) {
      fail(`fixture #${index} declaration differs from its frozen phase, mode, or expectation`);
    }
  }
  const registeredPaths = manifest.fixtures.map(({ path }) => path).sort();
  const sortedActualPaths = [...fixturePaths].sort();
  if (new Set(sortedActualPaths.map((path) => path.toLowerCase())).size !== sortedActualPaths.length) {
    fail('fixture paths collide case-insensitively');
  }
  if (JSON.stringify(registeredPaths) !== JSON.stringify(sortedActualPaths)) {
    fail('registered fixture inventory differs from tests/fixtures');
  }

  return manifest;
}

export function validatePackageDocument(packageDocument) {
  if (!packageDocument || typeof packageDocument !== 'object' || Array.isArray(packageDocument)) {
    fail('package.json root must be an object');
  }
  if (packageDocument.scripts?.['m0:check'] !== CANONICAL_PACKAGE_ALIAS) {
    fail(`package.json m0:check must be exactly ${CANONICAL_PACKAGE_ALIAS}`);
  }
  return packageDocument;
}

export function loadAndValidateManifest() {
  const stats = statSync(MANIFEST_PATH, { throwIfNoEntry: false });
  if (!stats?.isFile() || stats.size > MAX_MANIFEST_BYTES) {
    fail(`manifest must be a regular file no larger than ${MAX_MANIFEST_BYTES} bytes`);
  }

  let manifest;
  let packageDocument;
  try {
    manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8'));
    packageDocument = JSON.parse(readFileSync(PACKAGE_PATH, 'utf8'));
  } catch (error) {
    fail(`manifest or package.json is not strict JSON: ${error.message}`);
  }
  validatePackageDocument(packageDocument);
  return validateManifestDocument(manifest);
}

function platformName() {
  if (process.platform === 'win32') return 'windows';
  if (process.platform === 'linux') return 'linux';
  throw new Error(`M0 conformance is supported only on Linux and Windows, not ${process.platform}`);
}

export function runManifest(manifest) {
  const platform = platformName();
  for (const [index, command] of manifest.commands.entries()) {
    if (!command.platforms.includes(platform)) fail(`command #${index} does not cover ${platform}`);
    console.log(`\n[M0 ${index + 1}/${manifest.commands.length}] ${command.id}`);
    const result = spawnSync(command.executable, command.args, {
      cwd: WORKSPACE_ROOT,
      env: { ...process.env, ...(command.environment ?? {}) },
      shell: false,
      stdio: 'inherit',
      windowsHide: true,
    });
    if (result.error) throw new Error(`${command.id} could not start: ${result.error.message}`);
    if (result.status !== 0) throw new Error(`${command.id} failed with exit status ${result.status ?? 'unknown'}`);
  }
  console.log(`\nM0 conformance passed on ${platform}: ${manifest.commands.length} commands, ${manifest.coverage.length} registered proof suites.`);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    runManifest(loadAndValidateManifest());
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
