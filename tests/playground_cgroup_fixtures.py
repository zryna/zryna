"""Descriptor/ownership unit checks; no real cgroup, process or isolation acceptance."""

from contextlib import ExitStack
import os
import pathlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] /
                      "examples/playground/restricted"))
import cgroup
from cgroup import JobGroup, PolicyError


NATIVE_OPEN = os.open
NATIVE_CLOSE = os.close
NATIVE_STAT = os.stat
NATIVE_RMDIR = os.rmdir
DEFAULT = object()


def fd_count():
    return len(os.listdir("/proc/self/fd"))


def evidence(error):
    result = [error]
    if isinstance(error, BaseExceptionGroup):
        for nested in error.exceptions:
            result.extend(evidence(nested))
    cleanup = getattr(error, "cleanup", None)
    if cleanup is not None:
        result.extend(evidence(cleanup))
    return result


class Controls:
    """Only control-file data is mocked; directory handles and identities are real."""
    def __init__(self, root):
        self.root = root
        self.values = {}
        self.calls = []
        self.fault = lambda *_args: DEFAULT

    def __call__(self, fd, name, value=None):
        self.calls.append((name, value))
        override = self.fault(fd, name, value)
        if override is not DEFAULT:
            return override
        if value is not None:
            self.values[name] = str(value)
            return None
        if os.fstat(fd).st_ino == self.root.stat().st_ino:
            return {"cgroup.type": "domain", "cgroup.subtree_control": "cpu cpuset memory pids",
                    "cgroup.procs": "", "cpuset.cpus.effective": "0-3",
                    "cpuset.mems.effective": "0"}[name]
        if name in self.values:
            return self.values[name]
        return {"cpuset.cpus.effective": "0-1", "memory.events": "max 0\noom 0\noom_kill 0",
                "pids.events": "max 0", "cgroup.events": "populated 0"}[name]


class Fixture:
    def __init__(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="playground-unit-")
        self.mount = pathlib.Path(self.temporary.name)
        self.root = self.mount / "delegated"
        self.root.mkdir()
        self.controls = Controls(self.root)
        self.groups = []
        self.stack = ExitStack()
        mountinfo = f"10 1 0:1 / {self.mount} rw - cgroup2 cgroup rw\n"
        self.stack.enter_context(patch("cgroup.pathlib.Path.read_text", return_value=mountinfo))
        self.stack.enter_context(patch("cgroup.control", self.controls))
        self.stack.enter_context(patch("cgroup.os.open", side_effect=self.open))

    @staticmethod
    def open(name, flags, *args, **kwargs):
        # A real disposable descriptor satisfies the constructor's kill-file open probe.
        if name == "cgroup.kill":
            return NATIVE_OPEN("/dev/null", os.O_WRONLY)
        return NATIVE_OPEN(name, flags, *args, **kwargs)

    def create(self, clock=lambda: 0):
        group = JobGroup.__new__(JobGroup)
        self.groups.append(group)
        JobGroup.__init__(group, str(self.root), 536_870_912, 32, clock=clock)
        return group

    def close(self):
        self.stack.close()
        for group in self.groups:
            for attribute in ["fd", "root_fd"]:
                fd = getattr(group, attribute, None)
                if fd is not None:
                    NATIVE_CLOSE(fd)
                    setattr(group, attribute, None)
        self.temporary.cleanup()


@unittest.skipUnless(sys.platform == "linux", "Linux directory descriptors required")
class GroupFixtureCase(unittest.TestCase):
    def setUp(self):
        self.before = fd_count()
        self.fixtures = []

    def tearDown(self):
        for fixture in reversed(self.fixtures):
            fixture.close()
        self.assertEqual(fd_count(), self.before)

    def fixture(self):
        fixture = Fixture()
        self.fixtures.append(fixture)
        return fixture

