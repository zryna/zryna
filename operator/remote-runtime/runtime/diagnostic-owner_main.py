"""Fixed isolated interpreter entrypoint. No executable/URL/socket/path command selectors."""
import os
import select
import sys
import time

ROOT = os.path.dirname(os.path.abspath(__file__))
STDLIB = os.path.join(ROOT, 'lib', 'python3.12')
# Require the captured interpreter's private prefix; never fall back to ambient modules.
if sys.prefix != ROOT or sys.exec_prefix != ROOT:
    raise ValueError('UI private Python prefix required')
sys.path[:] = [ROOT, STDLIB, os.path.join(STDLIB, 'lib-dynload')]
from engine import decode
from plan import require, exact
from owner_lifecycle import Owner
from owner_relay import run, stop_deadline
import json


def control(value, deadline):
    raw = json.dumps(value, separators=(',', ':')).encode() + b'\n'
    require(len(raw) <= 2097152, 'UI bounded sanitized owner receipt')
    os.set_blocking(3, False)
    while raw:
        require(time.monotonic() < deadline, 'UI owner control deadline')
        ready = select.select([], [3], [], min(0.1, deadline - time.monotonic()))[1]
        if ready:
            count = os.write(3, raw)
            require(count > 0, 'UI owner control write')
            raw = raw[count:]


def main():
    deadline = time.monotonic() + 300
    raw = bytearray()
    while not raw.endswith(b'\n'):
        require(time.monotonic() < deadline and len(raw) <= 262144, 'UI bounded initial approval')
        if not select.select([0, 3], [], [], min(0.1, deadline - time.monotonic()))[0]:
            continue
        if select.select([3], [], [], 0)[0]:
            raise ValueError('UI initial control interrupted')
        data = os.read(0, 1)
        require(data, 'UI approval EOF')
        raw.extend(data)
    initial = decode(bytes(raw))
    exact(initial, ['version', 'approval', 'deadlineNs'])
    require(type(initial['version']) is int and initial['version'] == 1 and
            type(initial['deadlineNs']) is str and initial['deadlineNs'].isascii() and
            initial['deadlineNs'].isdigit() and 1 <= len(initial['deadlineNs']) <= 20,
            'UI internal monotonic startup envelope')
    selected_deadline = int(initial['deadlineNs'])
    require(time.monotonic_ns() < selected_deadline <= time.monotonic_ns() + 300000000000,
            'UI shared finite300second limit')
    deadline = min(deadline, selected_deadline / 1000000000)
    owner = Owner(initial['approval'])
    owner.engine.deadline = min(owner.engine.deadline, deadline)
    cleanup = {}
    try:
        # Parent learns exact recovery name before the first Engine mutation.
        control({'type': 'reserved', 'nonce': owner.nonce, 'name': owner.name}, deadline)
        stream = owner.start()
        control({'type': 'ready', 'nonce': owner.nonce, 'containerId': owner.cid,
                 'audit': owner.record['audit']['receipt']}, deadline)
        owner.record['channelTerminal'] = run(stream, deadline, cleanup)
    except BaseException as error:
        owner.record['failure'] = ('UI_DIAG_M022_' + error.args[0][16:] if type(error) is ValueError and len(error.args)==1 and type(error.args[0]) is str and len(error.args[0])==37 and error.args[0].startswith('UI-MOUNTS: M022_') and set(error.args[0][16:30])<=set('01') and error.args[0][30]=='_' and error.args[0][31:34] in ('OVL','EXT','XFS','BTR','OTH') and error.args[0][34]=='_' and error.args[0][35:] in ('HN','HS','RC') else {
    "UI-CGROUP: actual captured acquisition thread": "UI_DIAG_CGROUP_005",
    "UI-CGROUP: actual first membership equality passed": "UI_DIAG_CGROUP_007",
    "UI-CGROUP: actual observer PID establishes approved mount coordinate system": "UI_DIAG_CGROUP_003",
    "UI-CGROUP: actual read-only cgroup-v2 mount": "UI_DIAG_CGROUP_016",
    "UI-CGROUP: all retained leader epochs unchanged across candidates": "UI_DIAG_CGROUP_014",
    "UI-CGROUP: canonical full-root cgroup membership": "UI_DIAG_CGROUP_013",
    "UI-CGROUP: complete observer mount metadata": "UI_DIAG_CGROUP_010",
    "UI-CGROUP: exact full-root cgroup-v2 topology": "UI_DIAG_CGROUP_012",
    "UI-CGROUP: exact managed-cloud sibling Docker topology": "UI_DIAG_CGROUP_011",
    "UI-CGROUP: exact original container identity": "UI_DIAG_CGROUP_001",
    "UI-CGROUP: fixed fresh pre-capability acquisition": "UI_DIAG_CGROUP_009",
    "UI-CGROUP: original acquisition context": "UI_DIAG_CGROUP_004",
    "UI-CGROUP: original first-membership proof context": "UI_DIAG_CGROUP_006",
    "UI-CGROUP: original init epoch remains live": "UI_DIAG_CGROUP_008",
    "UI-CGROUP: retained original container identity": "UI_DIAG_CGROUP_015",
    "UI-CGROUP: retirement proof before original acquisition deadline": "UI_DIAG_CGROUP_020",
    "UI-CGROUP: single bounded acquisition window": "UI_DIAG_CGROUP_017",
    "UI-CGROUP: sole cgroup-v2 mount": "UI_DIAG_CGROUP_002",
    "UI-CGROUP: successful candidate is retained initial identity": "UI_DIAG_CGROUP_018",
    "UI-CGROUP: successful capture retains all original leader anchors": "UI_DIAG_CGROUP_019",
    "UI-KERNEL: actual bounded cgroup tasks include init": "UI_DIAG_KERNEL_012",
    "UI-KERNEL: actual cgroup fixed CPU set": "UI_DIAG_KERNEL_018",
    "UI-KERNEL: actual fixed UI numeric cgroup controls": "UI_DIAG_KERNEL_017",
    "UI-KERNEL: actual fixed scratch flags": "UI_DIAG_KERNEL_027",
    "UI-KERNEL: actual fixed scratch size": "UI_DIAG_KERNEL_028",
    "UI-KERNEL: actual namespace link syntax": "UI_DIAG_KERNEL_043",
    "UI-KERNEL: actual network interface counters": "UI_DIAG_KERNEL_029",
    "UI-KERNEL: all UI process identities stable through tree observation": "UI_DIAG_KERNEL_013",
    "UI-KERNEL: all UI processes descend from owned init": "UI_DIAG_KERNEL_040",
    "UI-KERNEL: all four initial credentials 65534": "UI_DIAG_KERNEL_025",
    "UI-KERNEL: all initial capabilities zero": "UI_DIAG_KERNEL_026",
    "UI-KERNEL: all original UI thread memberships stable through global observation": "UI_DIAG_KERNEL_014",
    "UI-KERNEL: bounded positive process identities": "UI_DIAG_KERNEL_004",
    "UI-KERNEL: continuously running original UI init": "UI_DIAG_KERNEL_016",
    "UI-KERNEL: finite shared owner deadline": "UI_DIAG_KERNEL_001",
    "UI-KERNEL: fixed single-thread UI ancestor": "UI_DIAG_KERNEL_006",
    "UI-KERNEL: initial NNP and seccomp": "UI_DIAG_KERNEL_007",
    "UI-KERNEL: initial fixed two-CPU affinity": "UI_DIAG_KERNEL_008",
    "UI-KERNEL: kernel first-extra-byte bound": "UI_DIAG_KERNEL_038",
    "UI-KERNEL: missing cgroup metadata is not group removal": "UI_DIAG_KERNEL_042",
    "UI-KERNEL: namespace observation deadline": "UI_DIAG_KERNEL_039",
    "UI-KERNEL: no IPv4 routes": "UI_DIAG_KERNEL_011",
    "UI-KERNEL: no IPv6 external destination or next hop": "UI_DIAG_KERNEL_030",
    "UI-KERNEL: no escaped or unreaped owned descendant in complete PPid inventory": "UI_DIAG_KERNEL_035",
    "UI-KERNEL: observation deadline": "UI_DIAG_KERNEL_002",
    "UI-KERNEL: only private loopback interface": "UI_DIAG_KERNEL_010",
    "UI-KERNEL: ordinary kernel metadata": "UI_DIAG_KERNEL_023",
    "UI-KERNEL: original UI init still exists": "UI_DIAG_KERNEL_022",
    "UI-KERNEL: original UI init unchanged through kernel capture": "UI_DIAG_KERNEL_019",
    "UI-KERNEL: original running init before detailed kernel observation": "UI_DIAG_KERNEL_020",
    "UI-KERNEL: owned UI cgroup fully empty": "UI_DIAG_KERNEL_037",
    "UI-KERNEL: private PID1 readback": "UI_DIAG_KERNEL_009",
    "UI-KERNEL: retained pre-audit Docker identity": "UI_DIAG_KERNEL_036",
    "UI-KERNEL: same original owned process in complete visible inventory": "UI_DIAG_KERNEL_034",
    "UI-KERNEL: same-group UI process": "UI_DIAG_KERNEL_031",
    "UI-KERNEL: same-group UI thread": "UI_DIAG_KERNEL_033",
    "UI-KERNEL: stable UI process identity": "UI_DIAG_KERNEL_032",
    "UI-KERNEL: stable bounded UI process tree": "UI_DIAG_KERNEL_015",
    "UI-KERNEL: stable process start identity": "UI_DIAG_KERNEL_003",
    "UI-KERNEL: stable provisional init identity": "UI_DIAG_KERNEL_021",
    "UI-KERNEL: unique numeric cgroup cleanup events": "UI_DIAG_KERNEL_041",
    "UI-KERNEL: unique process identities": "UI_DIAG_KERNEL_005",
    "UI-KERNEL: unique process status fields": "UI_DIAG_KERNEL_024",
    "UI-MOUNTS: actual filesystem device identity": "UI_DIAG_MOUNTS_012",
    "UI-MOUNTS: actual fixed UI proc mount exists": "UI_DIAG_MOUNTS_007",
    "UI-MOUNTS: actual proc proof matches present interface": "UI_DIAG_MOUNTS_027",
    "UI-MOUNTS: actual read-only cgroup2 mount": "UI_DIAG_MOUNTS_003",
    "UI-MOUNTS: acyclic complete mount ancestry": "UI_DIAG_MOUNTS_030",
    "UI-MOUNTS: all five fixed proc protections exist": "UI_DIAG_MOUNTS_018",
    "UI-MOUNTS: complete bounded mountinfo rows": "UI_DIAG_MOUNTS_009",
    "UI-MOUNTS: complete connected mount tree": "UI_DIAG_MOUNTS_023",
    "UI-MOUNTS: connected base outside stack": "UI_DIAG_MOUNTS_015",
    "UI-MOUNTS: exact Docker file masked by same-dev null tmpfs bind": "UI_DIAG_MOUNTS_022",
    "UI-MOUNTS: exact original container ID": "UI_DIAG_MOUNTS_001",
    "UI-MOUNTS: exact read-only bind of selected proc protection": "UI_DIAG_MOUNTS_019",
    "UI-MOUNTS: exact read-only remount of original proc": "UI_DIAG_MOUNTS_020",
    "UI-MOUNTS: missing sysrq interface requires actual same-container native ENOENT proof": "UI_DIAG_MOUNTS_026",
    "UI-MOUNTS: mount parent is actual path ancestor": "UI_DIAG_MOUNTS_024",
    "UI-MOUNTS: namespace root external parent": "UI_DIAG_MOUNTS_028",
    "UI-MOUNTS: no approved runtime image overmount": "UI_DIAG_MOUNTS_016",
    "UI-MOUNTS: no bounded scratch subtree overmount": "UI_DIAG_MOUNTS_005",
    "UI-MOUNTS: no child hidden under covered stack base": "UI_DIAG_MOUNTS_029",
    "UI-MOUNTS: no fixed fontconfig ancestor overmount": "UI_DIAG_MOUNTS_006",
    "UI-MOUNTS: no material subtree overmount": "UI_DIAG_MOUNTS_004",
    "UI-MOUNTS: no protected proc subtree overmount": "UI_DIAG_MOUNTS_017",
    "UI-MOUNTS: numeric mount identities": "UI_DIAG_MOUNTS_010",
    "UI-MOUNTS: only exact observed two-layer proc/null stacks": "UI_DIAG_MOUNTS_013",
    "UI-MOUNTS: present Docker host file is actually null-masked": "UI_DIAG_MOUNTS_025",
    "UI-MOUNTS: sole UI proc permits own sandbox namespace maps with fixed safety flags": "UI_DIAG_MOUNTS_008",
    "UI-MOUNTS: sole fixed dev tmpfs backing null masks": "UI_DIAG_MOUNTS_021",
    "UI-MOUNTS: sole runtime root mount": "UI_DIAG_MOUNTS_002",
    "UI-MOUNTS: unique acyclic same-target parent-child stack": "UI_DIAG_MOUNTS_014",
    "UI-MOUNTS: unique exact mount identity/path": "UI_DIAG_MOUNTS_011",
    "UI-PROC: actual complete observer PID namespace": "UI_DIAG_PROC_001",
    "UI-PROC: all original leaders descend from original init": "UI_DIAG_PROC_024",
    "UI-PROC: all original live leader identities in complete inventory": "UI_DIAG_PROC_016",
    "UI-PROC: bounded canonical process inventory": "UI_DIAG_PROC_020",
    "UI-PROC: bounded parent identity": "UI_DIAG_PROC_003",
    "UI-PROC: complete bounded task group": "UI_DIAG_PROC_006",
    "UI-PROC: complete exact process stat identity": "UI_DIAG_PROC_002",
    "UI-PROC: complete observer mount metadata": "UI_DIAG_PROC_013",
    "UI-PROC: complete process inventory finished before deadline": "UI_DIAG_PROC_005",
    "UI-PROC: complete stable live task epoch": "UI_DIAG_PROC_007",
    "UI-PROC: complete visible proc without hidden processes": "UI_DIAG_PROC_014",
    "UI-PROC: exact bounded task status": "UI_DIAG_PROC_023",
    "UI-PROC: exact first or proven-known last membership failure": "UI_DIAG_PROC_008",
    "UI-PROC: exact retired-task subset and task count delta": "UI_DIAG_PROC_010",
    "UI-PROC: finite complete proc traversal": "UI_DIAG_PROC_019",
    "UI-PROC: global inventory deadline": "UI_DIAG_PROC_004",
    "UI-PROC: known actual task state": "UI_DIAG_PROC_022",
    "UI-PROC: leader unchanged through retirement inventory": "UI_DIAG_PROC_018",
    "UI-PROC: no escaped or unreaped owned descendant": "UI_DIAG_PROC_017",
    "UI-PROC: only exact task open absence": "UI_DIAG_PROC_025",
    "UI-PROC: only witnessed retired first or proven-known last membership task": "UI_DIAG_PROC_009",
    "UI-PROC: original leader epoch and membership": "UI_DIAG_PROC_015",
    "UI-PROC: retired task still absent after complete inventory": "UI_DIAG_PROC_012",
    "UI-PROC: retirement group stable through inventory": "UI_DIAG_PROC_011",
    "UI-PROC: unique numeric process identity": "UI_DIAG_PROC_021"
}.get(error.args[0], type(error).__name__) if type(error) is ValueError and len(error.args) == 1 and type(error.args[0]) is str and len(error.args[0]) <= 256 else type(error).__name__)
    finally:
        try:
            if 'deadline' not in cleanup and select.select([3], [], [], 0)[0]:
                pending = os.read(3, 1024)
                if pending:
                    cleanup['deadline'] = stop_deadline(pending)
        except BaseException:
            owner.record['failure'] = owner.record['failure'] or 'UI invalid cleanup control'
        receipt = owner.stop(cleanup.get('deadline'))
        try:
            control({'type': 'closed', 'receipt': receipt}, owner.cleanup_deadline)
        except BaseException:
            pass
    return 0 if receipt['removed'] and not receipt['cleanupFailure'] else 70


if __name__ == '__main__':
    try:
        sys.exit(main())
    except BaseException as error:
        if isinstance(error, SystemExit):
            raise
        # No raw exception or stdin/stdio values enter stderr or evidence.
        os.write(2, b'PLAYGROUND-UI-OWNER-FAILED\n')
        sys.exit(70)
