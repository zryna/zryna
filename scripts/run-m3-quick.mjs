import { runM3 } from './lib/m3-gates.mjs';

try {
  runM3(false);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
