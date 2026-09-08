import { bytes, parseCanonical } from '../package-release/canonical.mjs';
import { validateFixture as validatePackageFixture } from '../package-release/validate.mjs';

const authenticatedAuthorities = new WeakSet();

function fail(detail) {
  throw new Error(`P361-IDENTITY: ${detail}`);
}

function compareAscii(left, right) {
  return Buffer.compare(Buffer.from(left, 'ascii'), Buffer.from(right, 'ascii'));
}

export function packageKey(pkg) {
  return `${pkg.id}\0${pkg.sourceSha256}`;
}

export function rolePackageKey(role, pkg) {
  return `${role === 'target/runtime' ? '0' : '1'}\0${packageKey(pkg)}`;
}

export function validatePackageAuthority(input) {
  const receipt = validatePackageFixture(input);
  const fixture = parseCanonical(input);
  const byId = new Map(fixture.lock.packages.map((pkg) => [pkg.id, pkg]));
  const identity = (id) => {
    const pkg = byId.get(id);
    return Object.freeze({ id: pkg.id, sourceSha256: pkg.sourceSha256 });
  };
  const authority = Object.freeze({
    lockSha256: receipt.lockSha256,
    rootPackage: identity(fixture.lock.root),
    packages: Object.freeze(fixture.lock.packages.map((pkg) => Object.freeze({
      graphRole: 'target/runtime',
      package: identity(pkg.id),
      dependencies: Object.freeze(pkg.dependencies.map((dependency) => Object.freeze({
        alias: dependency.alias,
        graphRole: 'target/runtime',
        package: identity(dependency.package),
      }))),
    }))),
  });
  authenticatedAuthorities.add(authority);
  return authority;
}

function edgeKey(edge) {
  return `${edge.role}\0${packageKey(edge.from)}\0${edge.alias}`;
}

function normalizeCycle(cycle) {
  const rotations = cycle.map((_, index) => [...cycle.slice(index), ...cycle.slice(0, index)]);
  rotations.sort((left, right) =>
    compareAscii(left.map(edgeKey).join('\u0001'), right.map(edgeKey).join('\u0001')));
  return rotations[0];
}

function shortestPath(adjacency, start, destination) {
  if (start === destination) return [];
  const queue = [{ node: start, path: [] }];
  const visited = new Set([start]);
  for (let index = 0; index < queue.length; index++) {
    const current = queue[index];
    for (const edge of adjacency.get(current.node) ?? []) {
      if (edge.to === destination) return [...current.path, edge];
      if (!visited.has(edge.to)) {
        visited.add(edge.to);
        queue.push({ node: edge.to, path: [...current.path, edge] });
      }
    }
  }
  return undefined;
}

function compareCycles(left, right) {
  if (left.length !== right.length) return left.length - right.length;
  return compareAscii(left.map(edgeKey).join('\u0001'), right.map(edgeKey).join('\u0001'));
}

function validateCycles(adjacency, edges) {
  const cycles = [];
  for (const edge of edges) {
    const path = shortestPath(adjacency, edge.to, rolePackageKey(edge.role, edge.from));
    if (path) cycles.push(normalizeCycle([edge, ...path]));
  }
  if (cycles.length === 0) return;
  cycles.sort(compareCycles);
  const witness = cycles[0].map((edge) => `${edge.from.id}:${edge.alias}`).join(' -> ');
  fail(`dependency cycle ${witness}`);
}

export function validateAuthenticatedPackageGraph(plan, packagesByKey, authority) {
  if (!authenticatedAuthorities.has(authority)) fail('a validated #168 package authority is required');
  if (plan.packages.some((pkg) => pkg.graphRole === 'host/build') ||
      plan.sources.some((source) => source.graphRole === 'host/build')) {
    fail('source-only v0 does not admit host/build package occurrences');
  }
  if (plan.packageLockSha256 !== authority.lockSha256) {
    fail('package lock digest differs from validated #168 authority');
  }
  if (!bytes(plan.rootPackage).equals(bytes(authority.rootPackage))) {
    fail('root package differs from validated #168 authority');
  }

  const adjacency = new Map([...packagesByKey.keys()].map((key) => [key, []]));
  const edges = [];
  for (const pkg of plan.packages) {
    const from = rolePackageKey(pkg.graphRole, pkg.package);
    for (const dependency of pkg.dependencies) {
      const edge = {
        role: pkg.graphRole,
        from: pkg.package,
        alias: dependency.alias,
        to: rolePackageKey(dependency.graphRole, dependency.package),
      };
      adjacency.get(from).push(edge);
      edges.push(edge);
    }
  }
  for (const list of adjacency.values()) list.sort((left, right) => compareAscii(edgeKey(left), edgeKey(right)));
  edges.sort((left, right) => compareAscii(edgeKey(left), edgeKey(right)));
  validateCycles(adjacency, edges);

  const root = rolePackageKey('target/runtime', plan.rootPackage);
  const reached = new Set([root]);
  const queue = [root];
  for (let index = 0; index < queue.length; index++) {
    for (const edge of adjacency.get(queue[index]) ?? []) {
      if (!reached.has(edge.to)) {
        reached.add(edge.to);
        queue.push(edge.to);
      }
    }
  }
  const orphan = [...packagesByKey.keys()].filter((key) => !reached.has(key)).sort(compareAscii)[0];
  if (orphan) fail(`unreachable package ${packagesByKey.get(orphan).package.id}`);
  if (!bytes(plan.packages).equals(bytes(authority.packages))) {
    fail('package pairs or alias edges differ from validated #168 authority');
  }
}
