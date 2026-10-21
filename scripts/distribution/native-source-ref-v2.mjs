// Source labels are observations of the selected commit, never caller substitutions.
import { requireValue } from './canonical.mjs';
import { nativeSourceRef } from './native-inventory-v2.mjs';
import { ordinaryBytes, pinBytes } from '../../tests/native-provider-public-install/commands.mjs';

export function selectNativeSourceRef({ head, attachedRef, env }) {
  requireValue(/^[a-f0-9]{40}$/.test(head), 'native selected source commit');
  if (attachedRef !== null) {
    requireValue(nativeSourceRef(attachedRef), 'native actual attached source branch');
    return { ref: attachedRef, commit: head, observation: 'actual-git-symbolic-ref', event: null };
  }
  requireValue(env.GITHUB_ACTIONS === 'true' && env.GITHUB_REPOSITORY === 'zryna/zryna'
    && typeof env.GITHUB_EVENT_PATH === 'string', 'detached native source needs actual GitHub event');
  const body = ordinaryBytes(env.GITHUB_EVENT_PATH, 2 * 1024 * 1024);
  const event = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(body));
  requireValue(event.repository?.full_name === 'zryna/zryna', 'native event source repository');
  let ref;
  if (env.GITHUB_EVENT_NAME === 'pull_request') {
    const selected = event.pull_request?.head;
    requireValue(selected?.repo?.full_name === 'zryna/zryna' && selected.sha === head
      && selected.ref === env.GITHUB_HEAD_REF, 'native actual pull-request head identity');
    ref = 'refs/heads/' + selected.ref;
  } else if (env.GITHUB_EVENT_NAME === 'push') {
    requireValue(event.after === head && event.ref === env.GITHUB_REF,
      'native actual push source identity');
    ref = event.ref;
  } else if (env.GITHUB_EVENT_NAME === 'workflow_dispatch') {
    requireValue(env.GITHUB_SHA === head && (!event.ref
      || event.ref === env.GITHUB_REF || 'refs/heads/' + event.ref === env.GITHUB_REF),
    'native actual dispatch source identity');
    ref = env.GITHUB_REF;
  } else requireValue(false, 'native detached event kind unsupported');
  requireValue(nativeSourceRef(ref), 'native actual detached source branch');
  return { ref, commit: head, observation: 'actual-github-event-head',
    event: { ...pinBytes(env.GITHUB_EVENT_PATH, body), name: env.GITHUB_EVENT_NAME } };
}
