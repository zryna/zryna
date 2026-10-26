"""One exact local backport disposition after its fixed actual owner, never an ignore API."""
import re
from dependency_audit_inputs import decode, encode, require, sha, snapshot, unchanged
from dependency_audit_node import node_call

PATCH = "scripts/dependency-patches/msal-node-6.0.1-auth-code.patch"
PATCH_SHA = "ebeed7ec9c0aa2c2d7fdbc0adae85bdd33ef2f99e5fc80b4f676fdd10afb7055"
UPSTREAM = "f550958586475dbfeb1cd44be14a2d653728850e"
PARENT = "reviewed MSAL6 authCode backport preserves all eleven network-denied CJS/ESM controls"
PINS = {
    "tests/msal-backport.test.mjs": "efbfc377a382001dd53cd61fc9ab2a1678856c6ac047fbeca581e88b73dd5f0d",
    "tests/msal-backport/materials.mjs": "566ab6405ca5408ca56e7ddf2a70fe8da93061375c870461b36f2235d7a3ef04",
    "tests/msal-backport/probe.mjs": "7bbc65ef077f943dd7d3523f458eedf587838799597ec0a01b1dbcee77045fed",
    "tests/msal-backport/network-deny.cjs": "9d3dc4ba260f30c23df39807201b007cf7d5b53d7c250f62ae5935c199f3d5d2",
    "tests/msal-backport/provenance.json": "c236c11e16b7983111840ea3f6051d1422864a3b8e3160906fd601a7c284818f",
    "scripts/stability-gates/process.mjs": "12391db1cca0420525756ad4c601b5f4ac3354ee763e077298ebfcbe827a9cfb",
    "scripts/stability-gates/linux-tree.mjs": "555c1d5d1b22b995952e4458d3be6cf09a44cf3b2492ddcaf8cafa20a1a772f3",
    "scripts/stability-gates/interruption.mjs": "9e4c2e9c5c7a4f2df480593bd0f7ffa2b863f610e756e618bbdda4743064bbc8",
    "scripts/stability-gates/input.mjs": "09e5212dc41e61536461a8e1a13655463427db974ad59e82944b0d01efe80da0",
    PATCH: PATCH_SHA,
}
_SEAL = object()


class _CompletedBackport:
    def __init__(self, record, seal):
        require(seal is _SEAL, "no decoded backport constructor")
        self._raw = encode(record)


def backport_record(value):
    require(isinstance(value, _CompletedBackport), "only same-invocation actual owner disposition")
    record = decode(value._raw)
    require(record["upstreamCommit"] == UPSTREAM and record["patchSha256"] == PATCH_SHA,
            "exact upstream/patch disposition only")
    return record


def validate_owner(raw, status, expected_names):
    require(isinstance(raw, bytes) and len(raw) <= 16 * 1024 * 1024, "bounded owner stdout")
    require(status["exitCode"] == 0 and status["signal"] is None and status["error"] is None,
            "actual owner status must pass")
    require(status["stderrBytes"] == 0, "owner stderr is not suppressed")
    require(status["stdoutBytes"] == len(raw) and status["stdoutSha256"] == sha(raw), "owner raw binding")
    text = raw.decode("utf-8").replace("\r\n", "\n")
    require(re.search(r"[\x00-\x08\x0b-\x1f\x7f-\x9f]", text) is None, "unambiguous TAP")
    require(text.startswith("TAP version 13\n") and "Bail out!" not in text, "complete TAP version")
    require(re.search(r"^\s*not ok\b", text, re.M) is None, "no failed parent/subtest")
    rows = re.findall(r"^(    )?ok ([0-9]+) - (.+)$", text, re.M)
    nested = [(int(number), name) for indent, number, name in rows if indent]
    parent = [(int(number), name) for indent, number, name in rows if not indent]
    require(nested == list(enumerate(expected_names, 1)) and parent == [(1, PARENT)], "all eleven exact SDK names once")
    require(re.findall(r"^(?:    )?1\.\.[0-9]+$", text, re.M) == ["    1..11", "1..1"], "exact nested plans")
    counters = re.findall(r"^# (tests|suites|pass|fail|cancelled|skipped|todo) ([0-9]+)$", text, re.M)
    require(len(counters) == 7 and dict(counters) == {"tests": "12", "suites": "0", "pass": "12",
        "fail": "0", "cancelled": "0", "skipped": "0", "todo": "0"}, "12 actual cases, no ignored/zero proof")


def qualify_backport(node, root, output):
    captured = []
    for path, expected in PINS.items():
        material = snapshot(root / path)
        require(material["sha256"] == expected, "reviewed backport/owner/process source identity: " + path)
        captured.append(material)
    provenance = decode(next(row["bytes"] for row in captured if row["path"].endswith("provenance.json")))
    require(provenance["upstream"]["commit"] == UPSTREAM, "exact reviewed upstream fix")
    names = provenance["expectedCaseNames"]
    require(isinstance(names, list) and len(names) == 11 and len(set(names)) == 11, "frozen eleven names")
    before = node_call(node, root, output, "--msal", "msal-material-before")
    require(before["format"] == "zryna.dependency-msal-materials.v1" and len(before["installed"]) == 7,
            "actual fixed helper material result")
    status = node_call(node, root, output, "--msal-control", "msal-owner-bridge")
    require(status["format"] == "zryna.dependency-msal-owner-status.v1", "fixed owner status schema")
    require(status["argv"][1:] == ["--test", "--test-reporter=tap", "tests/msal-backport.test.mjs"], "fixed owner argv")
    raw = snapshot(output / "msal-owner.stdout.log", 16 * 1024 * 1024)["bytes"]
    stderr = snapshot(output / "msal-owner.stderr.log", 16 * 1024 * 1024)["bytes"]
    require(not stderr and status["stderrSha256"] == sha(stderr), "actual empty owner stderr")
    validate_owner(raw, status, names)
    after = node_call(node, root, output, "--msal", "msal-material-after")
    require(after == before, "same installed reviewed materials before/after owner")
    for material in captured:
        unchanged(material)
    return _CompletedBackport({"upstreamCommit": provenance["upstream"]["commit"], "patchSha256": PATCH_SHA,
        "installed": before["installed"], "ownerArgv": status["argv"], "ownerStdoutSha256": sha(raw),
        "actualNodeTestsPassed": 12, "actualSDKControlsPassed": 11, "sourceAuthenticated": False}, _SEAL)
