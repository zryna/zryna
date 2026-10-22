// Synthetic frames and host doubles establish transport/lifetime contracts, never execution proof.
import { createController } from '../../examples/playground/restricted/controller.mjs';
import { sha256 } from '../../examples/playground/restricted/limits.mjs';
import { frame, source, witSha256, templateBytes } from './service.mjs';

export function observation(selection, value = 42) {
  return { revision: selection.revision, logical: selection.logical,
    componentSha256: selection.componentSha256, value };
}

export function controllerFixture(arity = 0) {
  const events = []; const timers = new Map(); const calls = []; const workers = [];
  const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64),
    policySha256: 'c'.repeat(64), bindingTemplateSha256: 'd'.repeat(64) };
  const host = {
    async compile(input) {
      const value = await frame(input, { exports: [{ logical: 'value',
        component: 'zryna-export-76616c7565', core: 'value', arity }] });
      return { token: '1'.repeat(64), bytes: value.bytes,
        receipt: { revision: input.revision, sourceSha256: value.metadata.sourceSha256,
          frameSha256: await sha256(value.bytes), compilerSha256: authority.compilerSha256,
          toolkitSha256: authority.toolkitSha256, policySha256: authority.policySha256 } };
    },
    async evaluate(selection, signal) {
      calls.push({ selection, signal });
      return { token: '2'.repeat(64), observation: observation(selection) };
    },
    beginEvaluation() { throw new Error('legacy worker watchdog must not be used'); },
    async finish(token, cancelled) { events.push({ token, cancelled }); },
  };
  class ForbiddenWorker {
    constructor() { workers.push(this); throw new Error('UI worker creation is forbidden'); }
  }
  const controller = createController({ host, authority, witSha256, templateBytes,
    evaluationMode: 'supervised-host', WorkerClass: ForbiddenWorker,
    schedule(fn, ms) { const token = Symbol(); timers.set(token, { fn, ms }); return token; },
    unschedule(token) { timers.delete(token); } });
  controller.edit(source);
  return { controller, host, events, timers, calls, workers };
}

export const turn = () => new Promise(resolve => setImmediate(resolve));
