"""Independent cleanup failures preserve primary evidence and release both handles."""

import sys
import unittest
from unittest.mock import patch

from playground_cgroup_fixtures import (
    DEFAULT, NATIVE_CLOSE, PolicyError, evidence, fd_count, GroupFixtureCase,
)


@unittest.skipUnless(sys.platform == "linux", "Linux directory descriptors required")
class CleanupEvidence(GroupFixtureCase):
    def test_close_attempts_both_handle_releases_and_retains_all_failure_evidence(self):
        fixture = self.fixture()
        group = fixture.create()
        primary = PolicyError("PLAYGROUND-KILL-FAILED")
        releases = []
        errors = [OSError("child close acknowledgement"), OSError("root close acknowledgement")]
        def kill(_fd, name, _value):
            if name == "cgroup.kill":
                raise primary
            return DEFAULT
        fixture.controls.fault = kill
        def close(fd):
            # Model a reported close error after release; both real descriptors are disposed.
            NATIVE_CLOSE(fd)
            releases.append(fd)
            raise errors[len(releases) - 1]
        before = fd_count()
        owned = [group.fd, group.root_fd]
        with patch("cgroup.os.close", side_effect=close):
            with self.assertRaises(PolicyError) as failure:
                group.close(lambda: self.fail("failed kill must not reap"), 2)
        self.assertIs(failure.exception, primary)
        self.assertEqual(releases, owned)
        self.assertTrue(all(error in evidence(primary) for error in errors))
        self.assertEqual(fd_count(), before - 2)
        self.assertTrue(group.path.is_dir())

    def test_constructor_primary_rollback_and_close_failures_remain_separate(self):
        fixture = self.fixture()
        primary = PolicyError("PLAYGROUND-LIMIT-WRITE-FAILED")
        cleanup = OSError("owned removal acknowledgement")
        close_errors = [OSError("child close acknowledgement"), OSError("root close acknowledgement")]
        releases = []
        owned = []
        def fault(fd, name, value):
            if name == "memory.max" and value is not None:
                owned.extend([fd, fixture.groups[-1].root_fd])
                raise primary
            return DEFAULT
        fixture.controls.fault = fault
        def close(fd):
            NATIVE_CLOSE(fd)
            if fd in owned:
                releases.append(fd)
                raise close_errors[len(releases) - 1]
        before = fd_count()
        with patch("cgroup.os.rmdir", side_effect=cleanup), patch("cgroup.os.close", side_effect=close):
            with self.assertRaises(PolicyError) as failure:
                fixture.create()
        self.assertIs(failure.exception, primary)
        self.assertIs(primary.cleanup, cleanup)
        self.assertTrue(all(error in evidence(primary) for error in close_errors))
        self.assertEqual(len(releases), 2)
        self.assertEqual(fd_count(), before)
        self.assertIsNotNone(fixture.groups[-1].name)
