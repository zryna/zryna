"""One fixed sequential private runtime attempt; no default qualification verdict."""
import datetime
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

spec = importlib.util.spec_from_file_location('runtime_support', Path(__file__).with_name('runtime_support.py'))
s = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def main():
    s.need(len(sys.argv) == 2, 'fixed runner manifest argument')
    manifest = s.decode(s.ordinary(sys.argv[1]))
    root = Path(manifest['approval']['root'])
    ctx, manifest, root = s.context_inputs(root / 'REMOTE_RUNTIME_CONTEXT.json', sys.argv[1])
    operators = Path(ctx['runtime']['operatorRoot'])
    bound = s.decode(s.ordinary(operators / 'BOUND_MANIFEST.json'))
    s.need(bound['source'] == ctx['source'] and bound['phase'] == 'qualification', 'actual bound source/phase')
    for row in bound['files'] + bound['inputs']:
        s.checked(row)
    registry = bound['jobs']
    names = ['parity', 'plain-keyboard', 'original-ui-owner', 'http-boundaries', 'session-isolation',
             'browser-faults', 'compiler-owner-faults', 'compiler-resources', 'pipe-boundaries',
             'storage-boundaries', 'material-denials', 'role-kernel-policy', 'host-death-and-owned-recovery']
    s.need([row['name'] for row in registry] == names and len(registry) == 13, 'fixed mature thirteen selections')
    output = Path(ctx['runtime']['outputRoot'])
    output.mkdir(mode=0o700)
    journal = output / 'execution'
    journal.mkdir(mode=0o700)
    s.need(all(not Path(row['result']).parent.exists() for row in registry), 'all original cohort roots unrun')
    rows, started = [], now()
    s.save(journal / 'INTENT.json', {'source': ctx['source'], 'manifest': s.pin(sys.argv[1]),
        'binding': s.pin(operators / 'BOUND_MANIFEST.json'), 'startedAt': started, 'noRetries': True,
        'fullQualification': False, 'scope': 'Original corrected13 selections; actual individual case review remains separate. Partial failing wrappers retain all available positives/failures; first owning failure stops without replay.'})
    deadline = time.monotonic() + 2400
    failure_phase = 'owning-cohorts'
    try:
        for index, job in enumerate(registry, 1):
            s.git_source(ctx['source'])
            s.need(time.monotonic() < deadline, 'original bounded whole operator window')
            for row in bound['approvals']:
                s.checked(row)
            prefix = journal / (f'{index:02}-' + job['name'])
            row = {'name': job['name'], 'argv': job['argv'], 'startedAt': now(),
                   'trustedOuterSeconds': job['seconds'], 'nativeCleanupEstablishedByOuterOwner': False}
            s.save(str(prefix) + '.START.json', row)
            print(json.dumps({'event': 'actual-cohort-start', 'name': job['name']}), flush=True)
            child = None
            with Path(str(prefix) + '.stdout').open('xb') as out, Path(str(prefix) + '.stderr').open('xb') as err:
                try:
                    # Inherit the runtime phase's owned group; the harness can terminate the whole group.
                    child = subprocess.Popen(job['argv'], cwd='/workspace/zryna', env=os.environ.copy(),
                                             stdin=subprocess.DEVNULL, stdout=out, stderr=err)
                    row['pid'] = child.pid
                    row['epoch'] = Path(f'/proc/{child.pid}/stat').read_text().rsplit(')', 1)[1].split()[19]
                    try:
                        row['exitCode'] = child.wait(timeout=min(job['seconds'], max(0.01, deadline-time.monotonic())))
                    except subprocess.TimeoutExpired:
                        row['timedOut'] = True
                        child.kill()
                        try:
                            row['exitCode'] = child.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            row['directChildReapUnknown'] = True
                finally:
                    if child is not None and child.poll() is None:
                        child.kill()
                        try:
                            child.wait(timeout=10)
                        except subprocess.TimeoutExpired:
                            row['directChildReapUnknown'] = True
            row.update(finishedAt=now(), stdout=s.pin(str(prefix)+'.stdout'), stderr=s.pin(str(prefix)+'.stderr'))
            result = Path(job['result'])
            if result.exists():
                row['result'] = s.pin(result)
                try:
                    row['observedStatus'] = s.decode(s.ordinary(result)).get('status')
                except BaseException as error:
                    row['resultParseFailure'] = str(error)
            row['owningCommandPassed'] = row.get('exitCode') == 0 and not row.get('timedOut') and row.get('observedStatus') == job['status']
            rows.append(row)
            s.save(str(prefix)+'.COMPLETE.json', row)
            s.need(row['owningCommandPassed'], 'first owning command failure; retained raw and no retry/default qualification')
        failure_phase = 'final-readonly-confirmation'
        # Separate read-only metadata phase after all thirteen original owning commands passed.
        helper = s.checked(bound['finalConfirmation'])
        s.need(helper == operators / 'runner' / 'readonly-confirmation.py', 'fixed approved final helper')
        final_spec = importlib.util.spec_from_file_location('actual_final_readonly_confirmation', helper)
        final_module = importlib.util.module_from_spec(final_spec)
        final_spec.loader.exec_module(final_module)
        confirmation = final_module.confirm(sys.argv[1])
        s.save(journal / 'RESULT.json', {'status': 'thirteen-original-owning-commands-completed', 'source': ctx['source'],
            'rows': rows, 'startedAt': started, 'finishedAt': now(), 'fullQualification': False, 'finalReadOnlyConfirmation': confirmation,
            'scope': 'Actual command outcomes only. Operator final GET-only metadata retained separately; independent68 semantic review and independent whole-proof confirmation remain required.'})
    except BaseException as error:
        s.save(journal / 'FAILURE.json', {'status': ('stopped-at-first-owning-failure-no-retry' if failure_phase == 'owning-cohorts' else 'final-readonly-confirmation-failed-after-thirteen-commands'), 'source': ctx['source'], 'failurePhase': failure_phase,
            'failure': {'type': type(error).__name__, 'message': str(error)}, 'rows': rows,
            'startedAt': started, 'finishedAt': now(), 'fullQualification': False})
        raise


if __name__ == '__main__':
    main()
