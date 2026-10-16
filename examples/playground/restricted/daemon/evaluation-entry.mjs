// Installed only at the fixed /app/evaluate.mjs image path.
import { runEvaluationExecutable } from './daemon/evaluate.mjs';

if (process.argv.length !== 2) {
  process.stderr.write('PLAYGROUND-EVALUATION-CONFIGURATION\n');
  process.exitCode = 1;
} else {
  runEvaluationExecutable().catch(error => {
    process.stderr.write(`${/^PLAYGROUND-[A-Z-]+$/.test(error?.message) ? error.message : 'PLAYGROUND-EVALUATION-HOST'}\n`);
    process.exitCode = 1;
  });
}
