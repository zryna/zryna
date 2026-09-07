import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import Ajv2020 from 'ajv/dist/2020.js';

export const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

const capabilityLimits = {
  clock: ['maxSubscriptions', 'maxTimers'],
  environment: ['maxEntries', 'maxTotalBytes'],
  filesystem: ['maxPreopens', 'maxOpenDescriptors'],
  network: ['maxAllowedEndpoints', 'maxConcurrentOperations'],
  randomness: ['maxBytesPerCall', 'maxBytesPerInstance'],
};

const profileTargets = {
  browser: 'browser',
  command: 'wasi-command',
  server: 'wasi-http-server',
};

function fail(code, message) {
  throw new Error(`${code}: ${message}`);
}

function equalArray(actual, expected, label) {
  if (actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
    fail('ZRYNA-C4000', `${label} must match the canonical order exactly`);
  }
}

function compileSchema(schema) {
  return new Ajv2020({ allErrors: true, strict: true }).compile(schema);
}

function parseJson(text, label) {
  try {
    return JSON.parse(text);
  } catch {
    fail('ZRYNA-C4000', `${label} is not valid JSON`);
  }
}

export function parseWitWorlds(source, maxBytes) {
  if (Buffer.byteLength(source, 'utf8') > maxBytes) fail('ZRYNA-C4004', 'WIT source exceeds maxWitBytes');
  if (source.includes('\0') || source.includes('\r')) fail('ZRYNA-C4000', 'WIT source must be LF-only UTF-8 text');
  const text = source.replace(/\/\/[^\n]*/g, '');
  const packageMatch = text.match(/^\s*package\s+([a-z][a-z0-9-]*:[a-z][a-z0-9-]*@[0-9]+\.[0-9]+\.[0-9]+);/);
  if (!packageMatch) fail('ZRYNA-C4000', 'WIT package declaration is missing or malformed');
  const worlds = new Map();
  const worldPattern = /world\s+([a-z][a-z0-9-]*)\s*\{([^{}]*)\}/g;
  for (const match of text.matchAll(worldPattern)) {
    const [, name, body] = match;
    if (worlds.has(name.toLowerCase())) fail('ZRYNA-C4000', `duplicate WIT world ${name}`);
    const world = { imports: [], exports: [] };
    for (const statement of body.split(';').map((item) => item.trim()).filter(Boolean)) {
      const item = statement.match(/^(import|export)\s+([a-z][a-z0-9-]*:[a-z][a-z0-9-]*\/[a-z][a-z0-9-]*@[0-9]+\.[0-9]+\.[0-9]+)$/);
      if (!item) fail('ZRYNA-C4000', `malformed WIT statement in world ${name}`);
      world[`${item[1]}s`].push(item[2]);
    }
    for (const direction of ['imports', 'exports']) {
      const folded = world[direction].map((value) => value.toLowerCase());
      if (new Set(folded).size !== folded.length) fail('ZRYNA-C4000', `duplicate ${direction} in world ${name}`);
    }
    worlds.set(name.toLowerCase(), world);
  }
  const remainder = text
    .replace(packageMatch[0], '')
    .replace(worldPattern, '')
    .trim();
  if (remainder) fail('ZRYNA-C4000', 'unsupported or malformed WIT top-level item');
  return { package: packageMatch[1], worlds };
}

export function validateRegistryObject(registry, witSource, schema) {
  const validate = compileSchema(schema);
  if (!validate(registry)) fail('ZRYNA-C4000', `registry schema failed: ${validate.errors[0].instancePath || '/'}`);
  equalArray(registry.profiles.map(({ id }) => id), ['browser', 'command', 'server'], 'profile ids');
  const parsed = parseWitWorlds(witSource, registry.bounds.maxWitBytes);
  if (parsed.package !== registry.wit.package) fail('ZRYNA-C4000', 'WIT package identity drifted');
  const digest = createHash('sha256').update(witSource).digest('hex');
  if (digest !== registry.wit.sha256) fail('ZRYNA-C4000', 'WIT source digest drifted');
  if (parsed.worlds.size !== registry.bounds.maxProfiles) fail('ZRYNA-C4004', 'world count must equal maxProfiles');

  for (const profile of registry.profiles) {
    equalArray(profile.capabilities.map(({ id }) => id), registry.capabilityOrder, `${profile.id} capability ids`);
    if (profile.capabilities.length !== registry.bounds.maxCapabilitiesPerProfile) {
      fail('ZRYNA-C4004', `${profile.id} capability count exceeds its bound`);
    }
    const suffix = `${registry.wit.package.slice(0, registry.wit.package.lastIndexOf('@'))}/${profile.id}@0.1.0`;
    if (profile.world !== suffix) fail('ZRYNA-C4000', `${profile.id} world identity drifted`);
    if (profile.target !== profileTargets[profile.id]) fail('ZRYNA-C4000', `${profile.id} target identity drifted`);
    const witWorld = parsed.worlds.get(profile.id);
    if (!witWorld) fail('ZRYNA-C4000', `WIT world ${profile.id} is missing`);
    equalArray(profile.imports, witWorld.imports, `${profile.id} imports`);
    equalArray(profile.exports, witWorld.exports, `${profile.id} exports`);
    if (profile.imports.length > registry.bounds.maxWorldImports) fail('ZRYNA-C4004', `${profile.id} imports exceed maxWorldImports`);

    const grantedImports = [];
    for (const capability of profile.capabilities) {
      equalArray(Object.keys(capability.limits), capabilityLimits[capability.id], `${profile.id}/${capability.id} limit keys`);
      if (capability.interfaces.length > registry.bounds.maxInterfacesPerCapability) {
        fail('ZRYNA-C4004', `${profile.id}/${capability.id} interfaces exceed their bound`);
      }
      const values = Object.values(capability.limits);
      if (capability.decision === 'denied') {
        if (capability.interfaces.length || values.some((value) => value !== 0)) {
          fail('ZRYNA-C4001', `${profile.id}/${capability.id} denial must have no interface or resource authority`);
        }
      } else {
        if (!capability.interfaces.length || values.some((value) => value === 0)) {
          fail('ZRYNA-C4000', `${profile.id}/${capability.id} grant must be explicit and bounded`);
        }
        grantedImports.push(...capability.interfaces);
      }
    }
    equalArray(profile.imports, grantedImports, `${profile.id} granted interface projection`);
  }
  return registry;
}

export function validateCapabilityRequest(request, registry, schema) {
  const validate = compileSchema(schema);
  if (!validate(request)) fail('ZRYNA-C4000', `request schema failed: ${validate.errors[0].instancePath || '/'}`);
  const profile = registry.profiles.find(({ id }) => id === request.profile);
  if (!profile) fail('ZRYNA-C4000', 'unknown profile');
  const requested = new Set(request.requests);
  equalArray(
    request.requests,
    registry.capabilityOrder.filter((id) => requested.has(id)),
    'request capabilities',
  );
  const grants = new Map(profile.capabilities.map((item) => [item.id, item]));
  for (const id of requested) {
    if (grants.get(id)?.decision !== 'granted') fail('ZRYNA-C4001', `${profile.id} denies ${id}`);
  }
  for (const [id, usage] of Object.entries(request.usage ?? {})) {
    if (!requested.has(id)) fail('ZRYNA-C4002', `usage for unrequested capability ${id}`);
    const capability = grants.get(id);
    for (const [name, value] of Object.entries(usage)) {
      if (!(name in capability.limits)) fail('ZRYNA-C4002', `${name} is not a ${id} limit`);
      if (value > capability.limits[name]) fail('ZRYNA-C4003', `${profile.id}/${id}/${name} exceeds its limit`);
    }
  }
  return { profile: profile.id, world: profile.world, granted: [...requested] };
}

export async function loadContract(root = workspaceRoot) {
  const [registryText, registrySchemaText, requestSchemaText] = await Promise.all([
    readFile(path.join(root, 'tests/wit-capability-profiles-v1.json'), 'utf8'),
    readFile(path.join(root, 'schemas/zryna-wit-capability-profiles-v1.schema.json'), 'utf8'),
    readFile(path.join(root, 'schemas/zryna-capability-request-v1.schema.json'), 'utf8'),
  ]);
  const registry = parseJson(registryText, 'registry');
  const registrySchema = parseJson(registrySchemaText, 'registry schema');
  const requestSchema = parseJson(requestSchemaText, 'request schema');
  const validateRegistryShape = compileSchema(registrySchema);
  if (!validateRegistryShape(registry)) {
    fail('ZRYNA-C4000', `registry schema failed: ${validateRegistryShape.errors[0].instancePath || '/'}`);
  }
  const witSource = await readFile(
    path.join(root, 'spec/wit/capability-profiles-v1/worlds.wit'),
    'utf8',
  );
  validateRegistryObject(registry, witSource, registrySchema);
  return { registry, witSource, registrySchema, requestSchema };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const { registry } = await loadContract();
  process.stdout.write(`${registry.schema}: ${registry.profiles.length} profiles validated\n`);
}
