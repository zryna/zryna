"""Run one owned command with init-style reaping in a Linux cloud container."""
import ctypes
import json
import os
import signal
import subprocess
import sys
import time

if len(sys.argv) < 2:
    raise SystemExit('Usage: run-with-subreaper.py <command> [arguments...]')
libc = ctypes.CDLL(None, use_errno=True)
if libc.prctl(36, 1, 0, 0, 0) != 0:
    raise OSError(ctypes.get_errno(), 'Cannot enable child subreaper')
command = subprocess.Popen(sys.argv[1:], start_new_session=True)
main_status = None
reaped = 0
forced = False
deadline = None

def forward_signal(number, _frame):
    global deadline
    if main_status is None:
        try:
            os.killpg(command.pid, number)
        except ProcessLookupError:
            pass
    else:
        deadline = time.monotonic()

signal.signal(signal.SIGTERM, forward_signal)
signal.signal(signal.SIGINT, forward_signal)
while True:
    try:
        pid, status = os.waitpid(-1, 0 if main_status is None else os.WNOHANG)
    except ChildProcessError:
        break
    if pid:
        if pid == command.pid:
            main_status = os.waitstatus_to_exitcode(status)
            command.returncode = main_status
            deadline = time.monotonic() + 5
        else:
            reaped += 1
        continue
    if time.monotonic() >= deadline:
        # Only children adopted by this runner are eligible for forced cleanup.
        children = open(f'/proc/self/task/{os.getpid()}/children').read().split()
        for child in children:
            try:
                descriptor = os.pidfd_open(int(child))
                try:
                    fields = open(f'/proc/{child}/stat').read().rsplit(')', 1)[1].split()
                    if int(fields[1]) == os.getpid():
                        signal.pidfd_send_signal(descriptor, signal.SIGKILL)
                        forced = True
                finally:
                    os.close(descriptor)
            except (FileNotFoundError, ProcessLookupError):
                pass
    time.sleep(0.005)
print(json.dumps({'child_exit_code': main_status, 'adopted_children_reaped': reaped,
                  'forced_cleanup': forced}), file=sys.stderr)
raise SystemExit(main_status if main_status and main_status > 0 else
                 (128 - main_status if main_status and main_status < 0 else (125 if forced else 0)))
