"""Descriptor/ownership unit checks; no real cgroup, process or isolation acceptance."""

from contextlib import ExitStack
import os
import sys
import unittest
from unittest.mock import patch

from playground_cgroup_fixtures import (
    DEFAULT, NATIVE_RMDIR, NATIVE_STAT, PolicyError, cgroup, fd_count, GroupFixtureCase,
)
from playground_cgroup_cleanup_cases import CleanupEvidence


@unittest.skipUnless(sys.platform == "linux", "Linux directory descriptors required")
class GroupLifetime(GroupFixtureCase):
    def test_real_owned_directory_limits_baseline_and_successful_close(self):
        fixture = self.fixture()
        group = fixture.create()
        path = group.path
        self.assertEqual(path.stat().st_mode & 0o777, 0o700)
        self.assertEqual(os.fstat(group.fd).st_ino, group.identity.st_ino)
        self.assertEqual(fixture.controls.values, {"memory.max": "536870912", "memory.swap.max": "0",
                         "pids.max": "32", "cpu.max": "200000 100000", "cpuset.mems": "0",
                         "cpuset.cpus": "0,1"})
        self.assertFalse(group.exceeded())
        fixture.controls.values["memory.events"] = "max 1\noom 0\noom_kill 0"
        self.assertTrue(group.exceeded())
        reaped = []
        group.close(lambda: reaped.append(True) or False, 2)
        self.assertEqual(reaped, [True])
        self.assertFalse(path.exists())
        self.assertTrue(group.closed)
        self.assertIsNone(group.name)
        self.assertIsNone(group.fd)
        self.assertIsNone(group.root_fd)
        count = len(fixture.controls.calls)
        group.close(lambda: self.fail("closed group reaped twice"), 2)
        self.assertEqual(len(fixture.controls.calls), count)

    def test_root_and_mount_handle_acquisition_failures_release_prior_handles(self):
        original = cgroup.directory
        for site in ["root", "mount"]:
            with self.subTest(site=site):
                fixture = self.fixture()
                before = fd_count()
                def directory(path):
                    if path == str(fixture.root if site == "root" else fixture.mount):
                        raise OSError("directory acquisition")
                    return original(path)
                with patch("cgroup.directory", side_effect=directory):
                    with self.assertRaisesRegex(OSError, "directory acquisition"):
                        fixture.create()
                self.assertEqual(fd_count(), before)
                self.assertEqual(list(fixture.root.iterdir()), [])

    def test_root_control_failures_never_create_job_and_release_root(self):
        for name, invalid in [("cgroup.type", "threaded"), ("cgroup.subtree_control", "cpu memory pids"),
                              ("cgroup.procs", "123"), ("cpuset.cpus.effective", "0"),
                              ("cpuset.mems.effective", "")]:
            with self.subTest(control=name):
                fixture = self.fixture()
                before = fd_count()
                fixture.controls.fault = lambda _fd, key, value: invalid if key == name and value is None else DEFAULT
                with self.assertRaises(PolicyError):
                    fixture.create()
                self.assertEqual(fd_count(), before)
                self.assertEqual(list(fixture.root.iterdir()), [])

    def test_mount_discovery_and_root_stat_failures_release_acquired_root(self):
        for site in ["fstat", "mountinfo", "wrong-filesystem"]:
            with self.subTest(site=site):
                fixture = self.fixture()
                before = fd_count()
                with ExitStack() as stack:
                    if site == "fstat":
                        stack.enter_context(patch("cgroup.os.fstat", side_effect=OSError("root stat")))
                    elif site == "mountinfo":
                        stack.enter_context(patch("cgroup.pathlib.Path.read_text", side_effect=OSError("mount discovery")))
                    else:
                        stack.enter_context(patch("cgroup.pathlib.Path.read_text", return_value=""))
                    with self.assertRaises((OSError, PolicyError)):
                        fixture.create()
                self.assertEqual(fd_count(), before)
                self.assertEqual(list(fixture.root.iterdir()), [])

    def test_mkdir_and_child_open_failures_rollback_exact_owned_directory(self):
        for site in ["mkdir", "open", "kill-open"]:
            with self.subTest(site=site):
                fixture = self.fixture()
                before = fd_count()
                def opened(name, flags, *args, **kwargs):
                    if site == "kill-open" and name == "cgroup.kill":
                        raise OSError("kill acquisition")
                    if site == "open" and name == fixture.groups[-1].name:
                        raise OSError("child acquisition")
                    return fixture.open(name, flags, *args, **kwargs)
                with patch("cgroup.os.open", side_effect=opened), ExitStack() as stack:
                    if site == "mkdir":
                        stack.enter_context(patch("cgroup.os.mkdir", side_effect=OSError("mkdir acquisition")))
                    with self.assertRaises(OSError):
                        fixture.create()
                self.assertEqual(fd_count(), before)
                self.assertEqual(list(fixture.root.iterdir()), [])

    def test_initial_stat_failure_retains_unproved_directory_and_separate_cleanup_evidence(self):
        fixture = self.fixture()
        before = fd_count()
        first = True
        primary = OSError("initial identity capture")
        def stat(path, *args, **kwargs):
            nonlocal first
            if first and str(path).startswith("playground-") and kwargs.get("dir_fd") is not None:
                first = False
                raise primary
            return NATIVE_STAT(path, *args, **kwargs)
        with patch("cgroup.os.stat", side_effect=stat), patch("cgroup.os.rmdir", wraps=NATIVE_RMDIR) as remove:
            with self.assertRaises(OSError) as failure:
                fixture.create()
            remove.assert_not_called()
        self.assertIs(failure.exception, primary)
        self.assertEqual(str(primary.cleanup), "PLAYGROUND-CGROUP-REPLACED")
        self.assertEqual(len(list(fixture.root.iterdir())), 1)
        self.assertIsNotNone(fixture.groups[-1].name)
        self.assertEqual(fd_count(), before)

    def test_limit_write_readback_effective_cpu_and_baseline_failures_rollback(self):
        for site in ["write", "readback", "effective", "baseline"]:
            with self.subTest(site=site):
                fixture = self.fixture()
                before = fd_count()
                def fault(fd, name, value):
                    child = os.fstat(fd).st_ino != fixture.root.stat().st_ino
                    if child and site == "write" and name == "memory.max" and value is not None:
                        raise OSError("limit write")
                    if child and site == "readback" and name == "memory.max" and value is None:
                        return "536870913"
                    if child and site == "effective" and name == "cpuset.cpus.effective":
                        return "0-3"
                    if child and site == "baseline" and name == "memory.events":
                        return "max invalid"
                    return DEFAULT
                fixture.controls.fault = fault
                with self.assertRaises((OSError, PolicyError, ValueError)):
                    fixture.create()
                self.assertEqual(fd_count(), before)
                self.assertEqual(list(fixture.root.iterdir()), [])

    def test_replacement_directory_is_preserved_on_close(self):
        fixture = self.fixture()
        group = fixture.create()
        saved = fixture.root / "retained-original"
        group.path.rename(saved)
        group.path.mkdir()
        foreign = group.path.stat()
        with patch("cgroup.os.rmdir", wraps=NATIVE_RMDIR) as remove:
            with self.assertRaisesRegex(PolicyError, "CGROUP-REPLACED"):
                group.close(lambda: False, 2)
            remove.assert_not_called()
        self.assertEqual(group.path.stat().st_ino, foreign.st_ino)
        self.assertEqual(saved.stat().st_ino, group.identity.st_ino)
        self.assertIsNotNone(group.name)
        self.assertFalse(group.closed)
        self.assertIsNone(group.fd)
        self.assertIsNone(group.root_fd)

    def test_replacement_between_identity_capture_and_open_rejects_before_controls(self):
        fixture = self.fixture()
        replaced = []
        def opened(name, flags, *args, **kwargs):
            if name == fixture.groups[-1].name and kwargs.get("dir_fd") is not None:
                path = fixture.root / name
                path.rename(fixture.root / "retained-original")
                path.mkdir()
                replaced.append(path.stat())
            return fixture.open(name, flags, *args, **kwargs)
        with patch("cgroup.os.open", side_effect=opened):
            with self.assertRaisesRegex(PolicyError, "CGROUP-REPLACED"):
                fixture.create()
        self.assertEqual(len(replaced), 1)
        self.assertFalse(any(value is not None for _name, value in fixture.controls.calls))
        group = fixture.groups[-1]
        self.assertEqual((fixture.root / group.name).stat().st_ino, replaced[0].st_ino)
        self.assertIsNotNone(group.name)
        self.assertIsNone(group.fd)
        self.assertIsNone(group.root_fd)

    def test_missing_or_noncanonical_population_data_cannot_prove_quiescence(self):
        for record in ["", "frozen 0", "populated 2", "populated 0\npopulated 0"]:
            with self.subTest(record=record):
                fixture = self.fixture()
                group = fixture.create(clock=lambda: 3)
                fixture.controls.fault = lambda _fd, name, _value: record if name == "cgroup.events" else DEFAULT
                with patch("cgroup.os.rmdir", wraps=NATIVE_RMDIR) as remove:
                    with self.assertRaises(PolicyError):
                        group.close(lambda: False, 2)
                    remove.assert_not_called()
                self.assertTrue(group.path.is_dir())
                self.assertIsNotNone(group.name)

    def test_close_requires_kill_population_zero_and_reap_proof_before_removal(self):
        fixture = self.fixture()
        group = fixture.create()
        sequence = []
        populations = iter(["populated 1", "populated 0", "populated 0"])
        children = iter([False, True, False])
        def fault(_fd, name, value):
            if name == "cgroup.kill":
                sequence.append("kill")
            if name == "cgroup.events":
                result = next(populations)
                sequence.append(result)
                return result
            return DEFAULT
        fixture.controls.fault = fault
        def reap():
            result = next(children)
            sequence.append(("children", result))
            return result
        def remove(*args, **kwargs):
            sequence.append("remove")
            return NATIVE_RMDIR(*args, **kwargs)
        with patch("cgroup.time.sleep"), patch("cgroup.os.rmdir", side_effect=remove):
            group.close(reap, 2)
        self.assertEqual(sequence, ["kill", ("children", False), "populated 1",
                         ("children", True), "populated 0", ("children", False), "populated 0", "remove"])

    def test_close_kill_reap_population_deadline_and_remove_failures_preserve_ownership(self):
        for site in ["kill", "reap", "population", "populated-deadline", "children-deadline", "rmdir"]:
            with self.subTest(site=site):
                fixture = self.fixture()
                group = fixture.create(clock=lambda: 3)
                name, identity = group.name, group.identity
                before = fd_count()
                primary = OSError(site)
                def fault(_fd, key, value):
                    if site == "kill" and key == "cgroup.kill":
                        raise primary
                    if key == "cgroup.events":
                        if site == "population":
                            raise primary
                        if site == "populated-deadline":
                            return "populated 1"
                    return DEFAULT
                fixture.controls.fault = fault
                def reap():
                    if site == "reap":
                        raise primary
                    return site == "children-deadline"
                with patch("cgroup.os.rmdir", side_effect=primary if site == "rmdir" else NATIVE_RMDIR) as remove:
                    with self.assertRaises((OSError, PolicyError)) as failure:
                        group.close(reap, 2)
                    if site != "rmdir":
                        remove.assert_not_called()
                if site.endswith("deadline"):
                    self.assertEqual(str(failure.exception), "PLAYGROUND-HOST-TEARDOWN")
                else:
                    self.assertIs(failure.exception, primary)
                self.assertEqual(group.name, name)
                self.assertIs(group.identity, identity)
                self.assertTrue(group.path.is_dir())
                self.assertFalse(group.closed)
                self.assertIsNone(group.fd)
                self.assertIsNone(group.root_fd)
                self.assertEqual(fd_count(), before - 2)



if __name__ == "__main__":
    unittest.main()
