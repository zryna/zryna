import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);

export function verifyProviderV4Result(required, result) {
  if (required === 'true' && result === 'success') return;
  if (required === 'false' && result === 'skipped') return;
  throw new Error(`invalid provider-v4 CI result: required=${required}, result=${result}`);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    verifyProviderV4Result(process.env.PROVIDER_V4_REQUIRED, process.env.PROVIDER_V4_RESULT);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
