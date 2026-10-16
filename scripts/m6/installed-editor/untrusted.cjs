'use strict';
const assert = require('node:assert/strict');

// A trust-disabled installed extension need not appear in this host's registry.
// Absence supplies neither a path match nor an inactive Extension object.
function observeUntrustedExtension(extension, expectedPath) {
  assert.equal(typeof expectedPath, 'string');
  if (extension === undefined) return { registered: false, active: null, path: null };
  const path = extension?.extensionPath;
  const active = extension?.isActive;
  const observation = {
    registered: true, valueType: extension === null ? 'null' : typeof extension,
    path: typeof path === 'string' ? path.slice(0, 4096) : null,
    pathType: typeof path, pathTruncated: typeof path === 'string' && path.length > 4096,
    active: typeof active === 'boolean' ? active : null, activeType: typeof active,
    expectedPath: expectedPath.slice(0, 4096), expectedPathTruncated: expectedPath.length > 4096,
  };
  try {
    assert(extension !== null && typeof extension === 'object', 'Malformed extension registry entry.');
    assert.equal(path, expectedPath, 'Registered extension must match the authenticated installed path.');
    assert.equal(active, false, 'Untrusted workspace must not activate Zryna.');
  } catch (error) {
    error.untrustedRegistryObservation = observation;
    throw error;
  }
  return { registered: true, active: false, path };
}
module.exports = { observeUntrustedExtension };
