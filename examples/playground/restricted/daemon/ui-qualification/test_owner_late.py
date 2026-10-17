"""Prospective late-task transactions: complete synthetic epochs, never Engine/browser."""
import errno
import unittest
from unittest import mock
import owner_cgroup as cgroup
import owner_kernel as kernel
import test_owner_acquisition as fixtures


class LateFixture:
    def __init__(self):
        self.case = fixtures.AcquisitionTests('runTest');self.case.setUp();self.case.retire = 999
        self.path, self.task = '/proc/323/cgroup', 323
        self.errors, self.failed_reads = [], []
        self.after_missing = lambda: None
        self.make_error = lambda path: fixtures.missing(path)
        self.multi = False

    def read(self, path, deadline, maximum=16384):
        candidate = cgroup._ACTIVE
        target = 322 + candidate.row['number'] if self.multi else self.task
        if candidate.site == 'TREE_LAST_MEMBERSHIPS' and path == f'/proc/{target}/cgroup':
            self.failed_reads.append((path, deadline));self.case.calls.append((path, deadline))
            threads = kernel.numbers(self.case.rows[fixtures.GROUP + '/cgroup.threads'])
            self.case.rows[fixtures.GROUP + '/cgroup.threads'] = ''.join(f'{pid}\n' for pid in threads if pid != target)
            self.case.rows[fixtures.GROUP + '/pids.current'] = str(len(threads) - 1) + '\n'
            for name in ('stat', 'status'):
                selected = f'/proc/{target}/{name}';self.case.rows[selected] = fixtures.missing(selected)
            self.after_missing()
            error = self.make_error(path);self.errors.append(error)
            raise error
        return self.case.observe(path, deadline, maximum)

    def run(self):
        return self.case.run_capture(change=self.read)


def late_receipt():
    fixture = LateFixture();fixture.run();return fixture.case.record


class LateTests(unittest.TestCase):
    def test_first_proved_live_then_late_singleton_retires_and_fresh_full_capture_passes(self):
        f = LateFixture();result = f.run();phase = f.case.record
        self.assertEqual([row['status'] for row in phase['attempts']], ['retired-task', 'accepted'])
        first = phase['attempts'][0]
        self.assertEqual(first['failure'], 'OPEN_ENOENT_LAST')
        self.assertEqual(first['priorTask'], {'ticks': '100', 'tgid': 322, 'stateBefore': 'S',
                                             'stateAfter': 'S', 'membershipVerified': True})
        self.assertEqual((first['before']['current'], first['after']['current']), (9, 8))
        self.assertEqual(f.failed_reads, [('/proc/323/cgroup', 101)])
        self.assertEqual(f.case.inspect_calls, 2)
        self.assertNotIn(323, result['processes']['threads'])
        self.assertEqual(first['leaders'], result['processes']['processes'])
        self.assertTrue(all(limit == 101 for _, limit in f.case.calls))
        # FIRST live stat/status bracket, two POST-failed-OPEN samples, and finalafterinventory sample.
        self.assertEqual(sum(path == '/proc/323/stat' for path, _ in f.case.calls), 5)
        self.assertEqual(sum(path == '/proc/323/status' for path, _ in f.case.calls), 5)
        self.assertIsNone(cgroup._ACTIVE)

    def test_first_commit_only_after_outer_equality_and_tuple_is_immutable(self):
        f = LateFixture();original_require, original_note = kernel.require, cgroup.note_first
        passed, known = set(), []
        def require(value, message):
            original_require(value, message)
            if message == 'same-group UI thread' and cgroup._ACTIVE.pending:
                passed.add(cgroup._ACTIVE.pending[0])
        def note(candidate, task):
            if task not in candidate.tree['processes']:
                self.assertIn(task, passed)
            original_note(candidate, task)
            if task in candidate.first:
                self.assertIs(type(candidate.first[task]), tuple);known.append(candidate.first[task])
        with mock.patch.object(kernel, 'require', side_effect=require), mock.patch.object(cgroup, 'note_first', side_effect=note):
            f.run()
        self.assertTrue(known)
        f = LateFixture();f.case.rows[f.path] = '0::/foreign\n'
        with mock.patch.object(cgroup, 'note_first', wraps=original_note) as note:
            with self.assertRaises(ValueError):f.run()
        self.assertFalse(any(call.args[1] == 323 for call in note.call_args_list))
        self.assertEqual(len(f.case.record['attempts']), 1)
        self.assertFalse(f.failed_reads)

    def test_no_prior_proof_and_leader_failure_are_fatal_without_reacquisition(self):
        f = LateFixture();original = cgroup.note_first
        with mock.patch.object(cgroup, 'note_first', side_effect=lambda candidate, task: None if task == 323 else original(candidate, task)):
            with self.assertRaises(FileNotFoundError) as observed:f.run()
        self.assertIs(observed.exception, f.errors[0]);self.assertEqual(f.case.inspect_calls, 1)
        self.assertIsNone(f.case.record['attempts'][0]['task'])
        f = LateFixture();f.task = 322
        with self.assertRaises(FileNotFoundError) as observed:f.run()
        self.assertIs(observed.exception, f.errors[0]);self.assertEqual(f.case.inspect_calls, 1)
        self.assertIsNone(f.case.record['attempts'][0]['task'])

    def test_present_replaced_task_or_non_open_absence_never_discards_candidate(self):
        for variant in ('present', 'replaced', 'status-esrch', 'stat-permission', 'READ', 'FSTAT', 'ESRCH', 'PERMISSION'):
            with self.subTest(variant=variant):
                f = LateFixture()
                def change():
                    if variant in ('present', 'replaced'):
                        f.case.rows['/proc/323/stat'] = fixtures.fixtures.proc_stat(323, tick='200' if variant == 'replaced' else '100')
                        f.case.rows['/proc/323/status'] = 'Pid:\t323\nTgid:\t322\n'
                    elif variant == 'status-esrch':f.case.rows['/proc/323/status'] = OSError(errno.ESRCH, 'fixture')
                    elif variant == 'stat-permission':f.case.rows['/proc/323/stat'] = PermissionError(errno.EACCES, 'fixture')
                f.after_missing = change
                if variant in ('READ', 'FSTAT'):
                    f.make_error = lambda path: fixtures.missing(path, variant)
                elif variant == 'ESRCH':f.make_error = lambda path: OSError(errno.ESRCH, 'fixture', path)
                elif variant == 'PERMISSION':f.make_error = lambda path: PermissionError(errno.EACCES, 'fixture', path)
                with self.assertRaises(OSError) as observed:f.run()
                self.assertIs(observed.exception, f.errors[0]);self.assertEqual(f.case.inspect_calls, 1)
                self.assertEqual(f.case.record['attempts'][0]['status'], 'fatal')

    def test_every_original_retirement_group_and_ancestry_guard_remains_mandatory(self):
        variants = ('count', 'multiple', 'added', 'leader', 'membership', 'init', 'escape', 'zombie', 'final-present')
        for variant in variants:
            with self.subTest(variant=variant):
                f = LateFixture()
                def change():
                    if variant == 'count':f.case.rows[fixtures.GROUP + '/pids.current'] = '9\n'
                    elif variant == 'multiple':
                        f.case.rows[fixtures.GROUP + '/cgroup.threads'] = '321\n322\n325\n326\n327\n328\n329\n';f.case.rows[fixtures.GROUP + '/pids.current'] = '7\n'
                    elif variant == 'added':f.case.rows[fixtures.GROUP + '/cgroup.threads'] = '321\n322\n324\n325\n326\n327\n328\n999\n'
                    elif variant == 'leader':f.case.rows['/proc/322/stat'] = fixtures.fixtures.proc_stat(322, tick='200')
                    elif variant == 'membership':f.case.rows['/proc/322/cgroup'] = '0::/foreign\n'
                    elif variant == 'init':f.case.rows['/proc/321/stat'] = fixtures.fixtures.proc_stat(321, tick='200')
                    elif variant in ('escape', 'zombie'):
                        f.case.inventory_extra[999] = {'pid':999,'ticks':'1','parent':322,'state':'Z' if variant == 'zombie' else 'S'}
                f.after_missing = change
                if variant == 'final-present':
                    original_read=f.read;reads=[0]
                    def read(path, deadline, maximum=16384):
                        if path == '/proc/323/stat':
                            reads[0] += 1
                            if reads[0] == 5:return fixtures.fixtures.proc_stat(323)
                        return original_read(path,deadline,maximum)
                    reader=read
                else:reader=f.read
                with self.assertRaises(FileNotFoundError) as observed:f.case.run_capture(change=reader)
                self.assertIs(observed.exception, f.errors[0]);self.assertEqual(f.case.inspect_calls, 1)
                self.assertEqual(f.case.record['attempts'][0]['status'], 'fatal')

    def test_three_attempt_ceiling_original_clock_and_fresh_docker_state(self):
        f = LateFixture();f.multi = True
        with self.assertRaises(FileNotFoundError) as observed:f.run()
        self.assertIs(observed.exception, f.errors[-1]);self.assertEqual(f.case.inspect_calls, 3)
        self.assertEqual([row['status'] for row in f.case.record['attempts']], ['retired-task'] * 3)
        self.assertEqual(f.case.record['status'], 'failed');self.assertTrue(all(limit == 101 for _, limit in f.failed_reads))
        f = LateFixture();f.after_missing = lambda: f.case.clock.__setitem__(0,101)
        with self.assertRaises(FileNotFoundError) as observed:f.run()
        self.assertIs(observed.exception, f.errors[0]);self.assertEqual(f.case.inspect_calls, 1)
        f = LateFixture();f.case.change_inspect = lambda: f.case.state['State'].update(OOMKilled=True)
        with self.assertRaises(ValueError):f.run()
        self.assertEqual([row['status'] for row in f.case.record['attempts']], ['retired-task','fatal'])
        self.assertEqual(f.case.original['ticks'], '100')
        # A new thread after the completed discarded witness requires a fresh complete proof.
        f = LateFixture()
        def current_epoch():
            threads = kernel.numbers(f.case.rows[fixtures.GROUP + '/cgroup.threads']) + [777]
            f.case.rows[fixtures.GROUP + '/cgroup.threads'] = ''.join(f'{pid}\n' for pid in threads)
            f.case.rows[fixtures.GROUP + '/pids.current'] = str(len(threads)) + '\n'
            f.case.rows['/proc/777/stat'] = fixtures.fixtures.proc_stat(777)
            f.case.rows['/proc/777/status'] = 'Pid:\t777\nTgid:\t322\n'
            f.case.rows['/proc/777/cgroup'] = fixtures.MEMBERSHIP
        f.case.change_inspect = current_epoch
        result = f.run()
        self.assertNotIn(777, f.case.record['attempts'][0]['after']['threads'])
        self.assertIn(777, result['processes']['threads'])
        self.assertIn(('/proc/777/cgroup', 101), f.case.calls)


if __name__ == '__main__':unittest.main()
