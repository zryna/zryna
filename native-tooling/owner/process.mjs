import { performance } from 'node:perf_hooks';
import { launchOwned } from './owner-client.mjs';

export function remaining(deadline) {
  const value = deadline - performance.now();
  if (value <= 0) throw new Error('Installed editor deadline exceeded.');
  return Math.ceil(value);
}
export async function command(executable, args, options) {
  const owner = await launchOwned(executable, args, { ...options, stage: options.stage ?? options.output });
  const result = await owner.finish();
  remaining(options.deadline);
  return result.stdout;
}
export async function startEditor(executable, args, options) {
  const owner = await launchOwned(executable, args, { ...options, name: 'editor-process',
    stage: options.stage ?? options.cwd });
  return { pid: owner.pid, failure: owner.failure, ended: owner.ended,
    async finish() {
      const natural = Math.min(options.deadline, performance.now() + 5000);
      while (!owner.ended() && !owner.failure() && performance.now() < natural) {
        await Promise.race([owner.closed, new Promise(resolve => setTimeout(resolve, 25))]);
      }
      return owner.finish({ stop: !owner.ended() });
    } };
}
