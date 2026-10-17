// Trusted operator harness contract. Adapters must be independently approved actual contained owners.
import { createParityRecorder } from './recorder.mjs';
import { createParityPlan, fixedCorpus } from './plan.mjs';
import { freeze } from './records.mjs';
import { boundedAdapterCall, validateAdapter } from './adapter.mjs';

export async function produceParity({ runCli, compilePlayground, evaluatePlayground, authority, witSha256, signal }) {
  for (const adapter of [runCli, compilePlayground, evaluatePlayground]) validateAdapter(adapter);
  const recorder = createParityRecorder({ authority, witSha256 });
  const plan = createParityPlan();
  const call = (adapter, input, kind, id, activeMs) =>
    boundedAdapterCall(adapter, input, { kind, id, activeMs, signal });
  try {
    for (const [index, entry] of fixedCorpus.entries()) {
      const build = plan.operations.find(value => value.corpusId === entry.id && value.kind === 'build');
      recorder.cli(build.id, await call(runCli, build, 'cli', build.id, 10000));
      const compilation = await call(compilePlayground, freeze({ corpusId: entry.id,
        sourceRequest: { version: 1, revision: index + 1, source: entry.source } }), 'compile', entry.id, 10000);
      await recorder.compilation(entry.id, compilation);
      for (const operation of plan.operations.filter(value => value.corpusId === entry.id && value.kind === 'run')) {
        recorder.cli(operation.id, await call(runCli, operation, 'cli', operation.id, 10000));
        const captured = await call(evaluatePlayground, freeze({ operationId: operation.id,
          corpusId: entry.id, logical: operation.logical, args: operation.args }), 'evaluate', operation.id, 5000);
        recorder.observation(operation.id, captured);
      }
    }
    return recorder.finish();
  } catch (caught) { recorder.abort(); throw caught; }
}
