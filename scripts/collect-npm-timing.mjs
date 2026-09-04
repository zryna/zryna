import { collectTiming, prepareTimingDirectory } from './lib/npm-timing.mjs';

try {
  if (process.argv.length !== 3) throw new Error();
  if (process.argv[2] === 'prepare') await prepareTimingDirectory(process.env.RUNNER_TEMP);
  else if (process.argv[2] === 'collect') {
    process.stdout.write(`${JSON.stringify(await collectTiming(process.env.RUNNER_TEMP))}\n`);
  } else throw new Error();
} catch {
  process.stderr.write('npm bootstrap timing unavailable\n');
  process.exitCode = 1;
}
