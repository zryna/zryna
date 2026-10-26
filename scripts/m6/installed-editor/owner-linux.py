"""Trusted test-only child owner. No user-source/remote command interface is exposed."""
import base64
import ctypes
import json
import os
import select
import signal
import sys
import time
LIMIT = 1024 * 1024
known = {}
root = None
root_status = None
out_size = [0, 0]
streams = {}
failures = []
metadata_bytes = 0
scope_reported = False
def event(kind, **values):
    print(json.dumps(dict(kind=kind, **values), separators=(",", ":")), flush=True)
def read_small(path, limit=8192):
    global metadata_bytes
    with open(path, "rb", buffering=0) as file:
        data = file.read(limit + 1)
    metadata_bytes += len(data)
    if metadata_bytes > 8 * LIMIT:
        raise RuntimeError("process metadata aggregate exceeded")
    if len(data) > limit:
        raise RuntimeError("process metadata bound exceeded")
    return data
def info(pid, with_image=True):
    raw = read_small(f"/proc/{pid}/stat")
    end = raw.rindex(b") ")
    fields = raw[end + 2:].decode("ascii").split()
    owner = os.stat(f"/proc/{pid}").st_uid
    try:
        image = os.readlink(f"/proc/{pid}/exe") if with_image else None
        if image is not None and len(image) > 4096:
            raise RuntimeError("process image path bound exceeded")
    except FileNotFoundError:
        image = None
    except PermissionError:
        if fields[0] != "Z":
            raise
        image = None
    return dict(pid=pid, start=fields[19], parent=int(fields[1]), group=int(fields[2]),
                state=fields[0], uid=owner, image=image)
def exited(item):
    poll = select.poll()
    poll.register(item["fd"], select.POLLIN)
    return bool(poll.poll(0))
def retain(row, reason):
    previous = known.get(row["pid"])
    if previous and previous["start"] == row["start"]:
        return False
    if previous:
        if not previous["gone"]:
            raise RuntimeError("owned PID reused before disappearance proof")
        os.close(previous["fd"])
    if len(known) >= 256:
        raise RuntimeError("owned identity bound exceeded")
    fd = os.pidfd_open(row["pid"], 0)
    try:
        after = info(row["pid"])
        if any(after[key] != row[key] for key in ("start", "uid", "image")):
            return False
        known[row["pid"]] = dict(row, fd=fd, reason=reason, gone=False)
        event("identity", **row, reason=reason)
        fd = None
        return True
    finally:
        if fd is not None:
            os.close(fd)
def scan(stage, deadline):
    global metadata_bytes, scope_reported
    metadata_bytes = 0
    names = [name for name in os.listdir("/proc") if name.isdecimal()]
    if len(names) > 4096:
        raise RuntimeError("process inventory bound exceeded")
    rows = {}
    for name in names:
        if time.monotonic() > deadline:
            raise RuntimeError("process observation deadline")
        pid = int(name)
        try:
            # Some managed kernels omit task/children. Only nonsensitive ancestry
            # metadata is scanned here; foreign executable visibility is not required.
            rows[pid] = info(pid, with_image=False)
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            if pid in known and not exited(known[pid]):
                raise RuntimeError("owned process ancestry inaccessible") from None
    if os.getpid() not in rows:
        raise RuntimeError("supervisor ancestry unavailable")
    attributed = {os.getpid()}
    for pid, item in known.items():
        row = rows.get(pid)
        if not item["gone"] and row and row["start"] == item["start"]:
            attributed.add(pid)
    changed = True
    while changed:
        changed = False
        if time.monotonic() > deadline:
            raise RuntimeError("process observation deadline")
        for pid, row in rows.items():
            if pid == os.getpid() or pid not in attributed and row["parent"] not in attributed:
                continue
            if row["uid"] != os.getuid():
                raise RuntimeError("owned process user identity changed")
            try:
                actual = info(pid)
            except (FileNotFoundError, ProcessLookupError):
                continue
            except PermissionError:
                raise RuntimeError("attributed owned executable inaccessible") from None
            if actual["start"] != row["start"]:
                continue
            old = known.get(pid)
            if old and old["start"] == actual["start"]:
                if old["parent"] != actual["parent"]:
                    old["parent"] = actual["parent"]
                    event("parent", pid=pid, start=old["start"], parent=actual["parent"])
                if actual["image"] and actual["image"].endswith(" (deleted)"):
                    raise RuntimeError("owned executing image deleted")
                if old["image"] and old["image"].startswith(stage + "/") and actual["image"] \
                        and not actual["image"].startswith(stage + "/"):
                    raise RuntimeError("owned stage image moved or changed outside approved stage: "
                                       f"pid={pid} start={actual['start']} before={old['image']!r} after={actual['image']!r}")
                if actual["image"] and old["image"] != actual["image"]:
                    old["image"] = actual["image"]
                    event("image", pid=pid, start=old["start"], image=actual["image"])
                continue
            try:
                retained = retain(actual, "subreaper-child" if actual["parent"] == os.getpid()
                                  else "owned-parent")
            except (FileNotFoundError, ProcessLookupError):
                continue
            if retained and pid not in attributed:
                attributed.add(pid)
                changed = True
    if not scope_reported:
        event("observation-scope", attributedRows=len(attributed), processRows=len(rows),
              unattributedMetadataUnavailable=len(names) - len(rows),
              foreignImagesNotQueried=len(rows) - len(attributed), subreaper=True)
        scope_reported = True
    for pid, item in known.items():
        if item["gone"] or not exited(item) or pid == root:
            continue
        row = rows.get(pid)
        if row and row["start"] == item["start"]:
            try:
                os.waitpid(pid, os.WNOHANG)
            except ChildProcessError:
                pass
        try:
            actual = info(pid, with_image=False)
        except (FileNotFoundError, ProcessLookupError):
            actual = None
        if actual is None or actual["start"] != item["start"]:
            item["gone"] = True
            event("gone", pid=pid, start=item["start"])
def kill_held():
    for item in reversed(list(known.values())):
        if not item["gone"] and not exited(item):
            try:
                signal.pidfd_send_signal(item["fd"], signal.SIGKILL, None, 0)
            except ProcessLookupError:
                pass
def main():
    global root, root_status
    config = json.loads(sys.stdin.buffer.readline(262145))
    if set(config) != {"program", "args", "cwd", "stage", "timeoutMs", "cleanupMs"}:
        raise RuntimeError("closed owner configuration required")
    if not all(isinstance(config[key], str) and 0 < len(config[key]) <= 32768
               for key in ("program", "cwd", "stage")) or not os.path.isabs(config["program"]):
        raise RuntimeError("fixed absolute owner paths required")
    if not isinstance(config["args"], list) or len(config["args"]) > 256 or not all(
            isinstance(arg, str) and len(arg) <= 32768 and "\0" not in arg for arg in config["args"]):
        raise RuntimeError("bounded owner arguments required")
    for key, maximum in (("timeoutMs", 600000), ("cleanupMs", 15000)):
        if type(config[key]) is not int or not 0 < config[key] <= maximum:
            raise RuntimeError("bounded owner deadline required")
    stage = os.path.realpath(config["stage"])
    if stage != config["stage"] or not os.path.isdir(stage):
        raise RuntimeError("ordinary private stage required")
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
    libc.prctl.restype = ctypes.c_int
    if libc.prctl(36, 1, 0, 0, 0) != 0:
        raise RuntimeError("private child-subreaper unavailable")
    probe = os.pidfd_open(os.getpid(), 0)
    os.close(probe)
    dependencies = {os.path.realpath(module.__file__) for module in sys.modules.values()
                    if getattr(module, "__file__", None) and os.path.isfile(module.__file__)}
    for line in read_small("/proc/self/maps", 262144).decode("utf-8").splitlines():
        path = line.split(None, 5)[-1]
        if path.startswith("/"):
            dependencies.add(os.path.realpath(path))
    event("ready", python=sys.version, runtime=sys.executable, pid=os.getpid(), subreaper=True,
          dependencies=sorted(dependencies))
    if sys.stdin.buffer.readline(64) != b'{"op":"start"}\n':
        raise RuntimeError("explicit owner start required")
    stdout_r, stdout_w = os.pipe2(os.O_CLOEXEC)
    stderr_r, stderr_w = os.pipe2(os.O_CLOEXEC)
    gate_r, gate_w = os.pipe2(os.O_CLOEXEC)
    root = os.fork()
    if root == 0:
        try:
            os.setsid()
            os.close(gate_w)
            os.dup2(stdout_w, 1); os.dup2(stderr_w, 2)
            null = os.open("/dev/null", os.O_RDONLY)
            os.dup2(null, 0)
            if os.read(gate_r, 1) != b'1':
                os._exit(126)
            os.close(gate_r)
            os.chdir(config["cwd"])
            os.closerange(3, 65536)
            os.execvpe(config["program"], [config["program"], *config["args"]], os.environ)
        except BaseException:
            os._exit(127)
    os.close(stdout_w); os.close(stderr_w); os.close(gate_r)
    startup_fd = os.pidfd_open(root, 0)
    try:
        retain(info(root), "startup-barrier-child")
    finally:
        os.close(startup_fd)
    event("started", pid=root)
    os.write(gate_w, b'1'); os.close(gate_w)
    streams.update({stdout_r: 0, stderr_r: 1})
    deadline = time.monotonic() + config["timeoutMs"] / 1000
    cleanup = None
    quiet = None
    stdin_open = True
    while True:
        now = time.monotonic()
        if now >= deadline and cleanup is None:
            failures.append("absolute execution deadline")
            cleanup = now + config["cleanupMs"] / 1000
        scan(stage, min(deadline if cleanup is None else cleanup, now + 2))
        if root_status is None and exited(known[root]):
            observed = os.waitid(os.P_PIDFD, known[root]["fd"], os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if observed:
                root_status = observed.si_status if observed.si_code == os.CLD_EXITED else 128 + observed.si_status
                event("root-exit", status=root_status, pid=root, start=known[root]["start"])
                cleanup = cleanup or now + config["cleanupMs"] / 1000
        if cleanup is not None:
            kill_held()
            alive = [item for pid, item in known.items() if pid != root and not item["gone"]]
            if root_status is not None and not alive and not streams:
                quiet = quiet or now
                if now - quiet >= 0.25:
                    os.waitpid(root, 0)
                    known[root]["gone"] = True
                    event("gone", pid=root, start=known[root]["start"])
                    event("complete", status=root_status, cleanupConfirmed=True, failures=failures)
                    return 0 if root_status == 0 and not failures else 1
            else:
                quiet = None
            if now >= cleanup:
                raise RuntimeError("owned process disappearance unconfirmed")
        readable, _, _ = select.select(([sys.stdin.fileno()] if stdin_open else []) + list(streams), [], [], 0.05)
        for fd in readable:
            if fd == sys.stdin.fileno():
                line = sys.stdin.buffer.readline(128)
                if line not in (b'{"op":"stop"}\n', b''):
                    raise RuntimeError("closed owner operation required")
                stdin_open = bool(line)
                cleanup = cleanup or time.monotonic() + config["cleanupMs"] / 1000
            else:
                data = os.read(fd, 65536)
                if not data:
                    os.close(fd); del streams[fd]
                    continue
                index = streams[fd]
                out_size[index] += len(data)
                if out_size[index] > LIMIT:
                    raise RuntimeError("owned output cap")
                event("data", stream=index, data=base64.b64encode(data).decode("ascii"))

try:
    sys.exit(main())
except BaseException as error:
    if isinstance(error, SystemExit):
        raise
    failures.append(str(error))
    emergency = time.monotonic() + 2
    while time.monotonic() < emergency:
        kill_held()
        try:
            if root is not None and os.waitpid(root, os.WNOHANG)[0] == root:
                break
        except ChildProcessError:
            break
        time.sleep(0.02)
    event("unknown", failures=failures,
          identities=[{key: value for key, value in item.items() if key != "fd"} for item in known.values()])
    sys.exit(70)
finally:
    for item in known.values():
        os.close(item["fd"])
