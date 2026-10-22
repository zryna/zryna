import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function verifyM4CIResult(required, routing, foundation, lanes) {
  if (routing !== 'success' || foundation !== 'success' || !Array.isArray(lanes) || lanes.length !== 3) {
    throw new Error('M4 CI: routing, full foundations and all three lane outcomes required');
  }
  if (required === 'true' && lanes.every(result => result === 'success')) return;
  if (required === 'false' && lanes.every(result => result === 'skipped')) return;
  throw new Error('M4 CI: required host lanes did not complete exactly');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    verifyM4CIResult(process.env.M4_REQUIRED, process.env.ROUTING_RESULT, process.env.M3_RESULT,
      [process.env.M4_CORE_RESULT, process.env.M4_BROWSER_RESULT, process.env.M4_INSTALLED_RESULT]);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
