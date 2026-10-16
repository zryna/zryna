"""Complete synthetic kernel epochs only: never Engine, native helper or browser execution."""
import errno
import unittest
from contextlib import ExitStack
from unittest import mock
import owner_kernel as kernel
import owner_cgroup as cgroup
import test_owner_kernel as fixtures

CID, GROUP, MEMBERSHIP = fixtures.CID, fixtures.GROUP, fixtures.MEMBERSHIP


def missing(path, syscall='OPEN'):
    error = FileNotFoundError(errno.ENOENT, 'synthetic disappearance', path)
    error._ui_kernel_syscall = syscall
    return error


class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.rows, self.state = fixtures.fixture()
        self.original = {'pid': 321, 'ticks': '100', 'cid': CID, 'membership': MEMBERSHIP,
                         'startedAt': self.state['State']['StartedAt']}
        self.record, self.calls, self.clock, self.error = {}, [], [100], None
        self.inspect_calls, self.change_inspect = 0, lambda: None
        self.threads = list(range(321, 330))
        self.rows[GROUP + '/cgroup.threads'] = ''.join(f'{pid}\n' for pid in self.threads)
        self.rows[GROUP + '/pids.current'] = '9\n'
        for task in self.threads[2:]:
            self.rows[f'/proc/{task}/stat'] = fixtures.proc_stat(task, parent=321)
            self.rows[f'/proc/{task}/status'] = f'Pid:\t{task}\nTgid:\t322\n'
            self.rows[f'/proc/{task}/cgroup'] = MEMBERSHIP
        self.retire = 323
        self.removed = False
        self.on_missing = lambda: None
        self.inventory_extra = {}

    def observe(self, path, deadline, maximum=16384):
        self.calls.append((path, deadline))
        if path in (f'/proc/{self.retire}/stat', f'/proc/{self.retire}/status'):
            raise missing(path)
        if path == f'/proc/{self.retire}/cgroup':
            self.rows[GROUP + '/cgroup.threads'] = ''.join(f'{pid}\n' for pid in self.threads if pid != self.retire)
            self.rows[GROUP + '/pids.current'] = '8\n'
            self.removed = True
            self.on_missing()
            if self.error is None:
                self.error = missing(path)
            raise self.error
        value = self.rows[path]
        if isinstance(value, BaseException):
            raise value
        return value

    def run_capture(self, change=None, anchors=None, deadline=399):
        def inspect():
            self.inspect_calls += 1
            if self.inspect_calls > 1:
                self.change_inspect()
            return self.state
        def namespace(path):
            subject, kind = path.split('/')[2], path.split('/')[-1]
            return kind + (':[101]' if subject == 'self' else ':[202]')
        def inventory(read, limit):
            return {**{pid: kernel.owner_proc.process(self.rows[f'/proc/{pid}/stat'], pid)
                       for pid in (321, 322)}, **self.inventory_extra}
        with ExitStack() as stack:
            for patch in [mock.patch.object(kernel, 'read', side_effect=change or self.observe),
                mock.patch.object(cgroup, 'mapping', return_value={'path': GROUP, 'profile': 'full-root-canonical-docker-v1'}),
                mock.patch.object(kernel.os, 'readlink', side_effect=namespace),
                mock.patch.object(kernel.owner_proc, 'inventory', side_effect=inventory),
                mock.patch.object(kernel.time, 'monotonic', side_effect=lambda: self.clock[0])]:
                stack.enter_context(patch)
            return kernel.acquire(inspect, CID, deadline, None, self.original,
                                  'POST_PROC_AUDIT', self.record, anchors)

    def test_actual_strict_full_capture_after_witnessed_nine_to_eight_epoch(self):
        result = self.run_capture()
        self.assertEqual(result['processes']['pidsCurrent'], 8)
        self.assertEqual([row['status'] for row in self.record['attempts']], ['retired-task', 'accepted'])
        row = self.record['attempts'][0]
        self.assertEqual((row['before']['current'], row['after']['current']), (9, 8))
        self.assertEqual(row['leaders'], result['processes']['processes'])
        self.assertTrue(row['noEscapedOwnedDescendants'])
        self.assertEqual(self.calls.count(('/proc/323/cgroup', 101)), 1)
        self.assertTrue(all(limit == 101 for _, limit in self.calls))
        self.assertIsNone(cgroup._ACTIVE)
        self.assertEqual(self.inspect_calls, 2)

    def test_every_fresh_candidate_inspects_current_docker_state_and_rejects_changed_flags(self):
        for key, value in [('Running', False), ('Paused', True), ('Restarting', True), ('OOMKilled', True),
                           ('Pid', 322), ('StartedAt', 'replacement-start')]:
            with self.subTest(key=key):
                self.setUp()
                self.change_inspect = lambda: self.state['State'].update({key: value})
                with self.assertRaises(ValueError):
                    self.run_capture()
                self.assertEqual(self.inspect_calls, 2)
                self.assertEqual([row['status'] for row in self.record['attempts']], ['retired-task', 'fatal'])
                self.assertEqual(self.original['pid'], 321)
        for key, value in [('RestartCount', 1), ('Id', 'f' * 64)]:
            self.setUp();self.change_inspect = lambda: self.state.update({key: value})
            with self.assertRaises(ValueError):
                self.run_capture()
            self.assertEqual(self.inspect_calls, 2)

    def test_permission_read_fstat_and_present_task_are_fatal_without_reacquisition(self):
        for error in [PermissionError(errno.EACCES, 'synthetic'), missing('/proc/323/cgroup', 'READ'),
                      missing('/proc/323/cgroup', 'FSTAT'), missing('/proc/999/cgroup')]:
            with self.subTest(error=type(error).__name__, syscall=getattr(error, '_ui_kernel_syscall', None)):
                self.setUp();self.error = error
                with self.assertRaises(type(error)) as observed:
                    self.run_capture()
                self.assertIs(observed.exception, error)
                self.assertEqual(len(self.record['attempts']), 1)
                self.assertEqual(self.record['status'], 'failed')
        self.setUp()
        def present(path, limit, maximum=16384):
            if path == '/proc/323/stat':
                return fixtures.proc_stat(323)
            if path == '/proc/323/status':
                return 'Pid:\t323\nTgid:\t322\n'
            return self.observe(path, limit, maximum)
        with self.assertRaises(FileNotFoundError):
            self.run_capture(present)
        self.assertEqual(len(self.record['attempts']), 1)

    def test_subset_counts_leaders_membership_and_global_escape_are_mandatory(self):
        changes = [lambda: self.rows.update({GROUP + '/pids.current': '9\n'}),
                   lambda: self.rows.update({GROUP + '/pids.current': '7\n', GROUP + '/cgroup.threads': '321\n322\n325\n326\n327\n328\n329\n'}),
                   lambda: self.rows.update({GROUP + '/cgroup.procs': '321\n324\n'}),
                   lambda: self.rows.update({'/proc/322/stat': fixtures.proc_stat(322, tick='101')}),
                   lambda: self.rows.update({'/proc/322/cgroup': '0::/foreign\n'}),
                   lambda: self.inventory_extra.update({999: {'pid': 999, 'ticks': '1', 'parent': 322, 'state': 'S'}}),
                   lambda: self.inventory_extra.update({999: {'pid': 999, 'ticks': '1', 'parent': 322, 'state': 'Z'}})]
        for change in changes:
            with self.subTest(change=changes.index(change)):
                self.setUp();self.on_missing = change
                with self.assertRaises(FileNotFoundError) as observed:
                    self.run_capture()
                self.assertIs(observed.exception, self.error)
                self.assertEqual(len(self.record['attempts']), 1)

    def test_retained_init_and_prior_leader_epochs_cannot_be_adopted(self):
        for change in [{'ticks': 'later'}, {'pid': 322}, {'startedAt': 'later'}, {'cid': 'f' * 64},
                       {'membership': '0::/foreign\n'}]:
            with self.subTest(change=change):
                self.setUp();self.original.update(change)
                with self.assertRaises(ValueError):
                    self.run_capture()
                self.assertEqual(self.record['status'], 'failed')
        self.setUp()
        with self.assertRaises(ValueError):
            self.run_capture(anchors=[{'pid': 321, 'ticks': '100', 'parent': 10},
                                      {'pid': 322, 'ticks': 'changed', 'parent': 321}])
        self.assertFalse(self.removed)

    def test_original_deadline_is_not_renewed_and_original_missing_exception_survives_expiry(self):
        self.on_missing = lambda: self.clock.__setitem__(0, 101)
        with self.assertRaises(FileNotFoundError) as observed:
            self.run_capture()
        self.assertIs(observed.exception, self.error)
        self.assertEqual(len(self.record['attempts']), 1)
        self.setUp()
        self.run_capture(deadline=100.25)
        self.assertTrue(all(limit == 100.25 for _, limit in self.calls))

    def test_unrelated_zombies_are_counted_without_waiving_owned_zombies(self):
        self.inventory_extra = {999: {'pid': 999, 'ticks': '1', 'parent': 1, 'state': 'Z'}}
        result = self.run_capture()
        self.assertEqual(self.record['attempts'][0]['unrelatedZombies'], 1)
        self.assertEqual(result['processes']['childObservation']['unrelatedZombiesObserved'], 1)
        self.setUp();self.on_missing = lambda: self.rows.update({'/proc/322/stat': fixtures.proc_stat(322, state='Z')})
        with self.assertRaises(FileNotFoundError):
            self.run_capture()

    def test_post_failure_supplement_error_preserves_original_exception_and_never_retries(self):
        def change(path, limit, maximum=16384):
            if self.removed and path == '/proc/323/status':
                raise PermissionError(errno.EACCES, 'synthetic supplemental failure')
            return self.observe(path, limit, maximum)
        with self.assertRaises(FileNotFoundError) as observed:
            self.run_capture(change)
        self.assertIs(observed.exception, self.error)
        self.assertEqual(len(self.record['attempts']), 1)


    def test_three_attempt_ceiling_and_fresh_complete_candidates(self):
        calls, errors = [], []
        leaders = [{'pid': 321, 'ticks': '100', 'parent': 10}, {'pid': 322, 'ticks': '100', 'parent': 321}]
        def strict(limit, candidate):
            calls.append(limit)
            task = 400 + len(calls)
            cgroup.note_tree(candidate, 321, GROUP, MEMBERSHIP, [321, 322], [321, 322, task], 3,
                             {leader['pid']: leader for leader in leaders})
            error = missing(f'/proc/{task}/cgroup')
            errors.append(error)
            candidate.error = error
            candidate.row.update(task=task, failure='OPEN_ENOENT_FIRST',
                targetBefore={'statAbsent': True, 'statusAbsent': True, 'state': None, 'ticks': None, 'tgid': None},
                targetAfter={'statAbsent': True, 'statusAbsent': True, 'state': None, 'ticks': None, 'tgid': None})
            raise error
        def prove(read, fields, numbers, original, tree, error, row, limit):
            row.update(after={'processes': [321, 322], 'threads': [321, 322], 'current': 2},
                       leadersStable=True, noEscapedOwnedDescendants=True, visibleProcesses=2, unrelatedZombies=0)
        with mock.patch.object(cgroup.time, 'monotonic', return_value=100), \
                mock.patch.object(cgroup, '_init_epoch'), mock.patch.object(cgroup.owner_proc, 'retired_task', side_effect=prove):
            with self.assertRaises(FileNotFoundError) as observed:
                cgroup.acquire(strict, self.observe, kernel.fields, kernel.numbers, self.original,
                               CID, 399, 'POST_PROC_AUDIT', self.record)
        self.assertIs(observed.exception, errors[-1])
        self.assertEqual(calls, [101, 101, 101])
        self.assertEqual([row['number'] for row in self.record['attempts']], [1, 2, 3])
        self.assertEqual(self.record['status'], 'failed')
        self.assertIsNone(cgroup._ACTIVE)

    def test_last_membership_missing_with_present_task_never_qualifies_for_acquisition(self):
        def change(path, limit, maximum=16384):
            if path in ('/proc/323/stat', '/proc/323/status'):
                return self.rows[path]
            if path == '/proc/323/cgroup':
                if sum(call[0] == path for call in self.calls):
                    raise missing(path)
                self.calls.append((path, limit))
                return MEMBERSHIP
            return self.observe(path, limit, maximum)
        with self.assertRaises(FileNotFoundError):
            self.run_capture(change)
        self.assertEqual(len(self.record['attempts']), 1)
        self.assertEqual(self.record['attempts'][0]['task'], 323)
        self.assertEqual(self.record['attempts'][0]['failure'], 'OPEN_ENOENT_LAST')
        self.assertFalse(self.record['attempts'][0]['targetBefore']['statAbsent'])

if __name__ == '__main__':
    unittest.main()
