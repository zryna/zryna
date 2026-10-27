// Read-only platform linkage. This command cannot admit raw measurements or create thresholds.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical, readSafe } from './input.mjs';
import { linkReviewedPerformanceRun, inspectPerformancePlatformLink } from './cli-performance-platform-link.mjs';

export function platformArguments(args) {
  assert(args.length === 6 && args[0] === '--expectations' && args[2] === '--expectations-sha256'
    && args[4] === '--output' && /^[a-f0-9]{64}$/u.test(args[3]),
  'usage: verify-cli-performance-platform.mjs --expectations PATH --expectations-sha256 SHA256 --output NEW_FILE');
  const input = resolve(args[1]), output = resolve(args[5]);
  assert.notEqual(input, output, 'S418 hosted: preserve the reviewed input');
  return { input, output, digest: args[3] };
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { input, output, digest } = platformArguments(process.argv.slice(2));
    const bytes = readSafe(dirname(input), basename(input));
    const key = await linkReviewedPerformanceRun(bytes, digest);
    writeFileSync(output, canonical(inspectPerformancePlatformLink(key)), { flag: 'wx', mode: 0o600 });
    process.stdout.write('Platform jobs and artifact metadata linked; raw artifacts and performance remain unqualified.\n');
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
