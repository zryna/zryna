// Imported only from the complete independently verified private source snapshot.
import { createService } from '../service.mjs';
import { createDaemonBridge } from './bridge.mjs';
import { createLauncherHost } from './launcher-adapter.mjs';

export async function createCore({ bridgePolicy, authority, witSha256, templateBytes, resources }) {
  const bridge = createDaemonBridge(bridgePolicy);
  const host = createLauncherHost(bridge);
  let service;
  try { service = createService({ host, authority, witSha256, templateBytes, resources,
    evaluationMode: 'supervised-host' }); }
  catch (primary) {
    try { await host.close(); } catch (cleanup) { throw new AggregateError([primary, cleanup], 'PLAYGROUND-SOURCE-CLEANUP'); }
    throw primary;
  }
  let launched = false;
  let stopped = false;
  let listening = false;
  let launchUrl = null;
  let stopping = null;
  return Object.freeze({
    async launch() {
      if (launched || stopped) throw new Error('PLAYGROUND-SOURCE-STATE');
      launched = true;
      const result = await service.listen();
      listening = true;
      if (!/^http:\/\/127\.0\.0\.1:[1-9][0-9]*$/.test(result.origin)) {
        await service.close(); throw new Error('PLAYGROUND-SOURCE-LISTENER');
      }
      launchUrl = result.url;
      return Object.freeze({ origin: result.origin, evaluationMode: 'supervised-host' });
    },
    takeLaunchUrl() {
      if (!launched || stopped || !launchUrl) throw new Error('PLAYGROUND-SOURCE-STATE');
      const result = launchUrl; launchUrl = null; return result;
    },
    recovery() { return host.recovery(); },
    close() {
      if (stopping) return stopping;
      stopped = true; launchUrl = null;
      stopping = (async () => {
        const errors = [];
        if (listening) { try { await service.close(); } catch (error) { errors.push(error); } }
        try { await host.close(); } catch (error) { errors.push(error); }
        if (errors.length === 1) throw errors[0];
        if (errors.length) throw new AggregateError(errors, 'PLAYGROUND-SOURCE-CLEANUP');
      })();
      return stopping;
    },
  });
}
