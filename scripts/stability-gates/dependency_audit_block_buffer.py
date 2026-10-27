"""Exact reviewed original-source disposition, after a current owned regression run."""
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import weakref
from dependency_audit_inputs import decode, encode, require, retain, sha, snapshot, unchanged

ADVISORY = "GHSA-qwgh-2vcv-g2f7"
DETAIL_SHA = "1520ee63f9e15ec2ecff019ac2e1f86d6b416985b7a584f6604cc39184bb45c5"
QUERY = {"package": {"ecosystem": "crates.io", "name": "block-buffer"}, "version": "0.10.4"}
CHECKSUM = "3078c7629b62d3f0439517fa394996acacc5cbc91c5a20d8c658e77abd503a71"
SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
ROOT_PINS = {
    "Cargo.toml": (3384, "765ba86bff634ffa0dfebf662c9fc3be00585b0b053c6b6a3e9ec48368e2e568"),
    "Cargo.lock": (52083, "a128e241fa1968308f2d5e2e828c94c29c413e718a7b96368e7a4032ac019751"),
}
REGISTRY_PINS = {
    "Cargo.toml": (917, "c76e4bd298cd6223882238449e31939d75613e82757ca87e249df843decf5f9a"),
    "src/lib.rs": (10700, "b437252d389042d894b1ef17859b043e827d2bd2a1bca4b5311c67dff565274d"),
    "src/sealed.rs": (2159, "9327ee3e567ae779b83c2b7f4f3ff56fcb2117163e023e7b0e37759e7e8d1f5c"),
}
TESTS = ["every_partial_position_and_input_byte_preserves_cursor_on_panic",
         "exact_advisory_eager_panic_keeps_separate_cursor",
         "generator_and_padding_panics_keep_valid_separate_cursors"]
# Complete reviewed source closure of the actual regression owner.
PINS = {'scripts/stability-gates/dependency-block-buffer-owner.mjs': '7b94a9164647279c07103003045c146c12a21e49dbe69bf0c2fcf942e1072313',
        'tests/dependency-audit/block-buffer-owner.rs': '0c5b8fd9c0eea321d89c921ec185e44627238b7af31d9e2a54268102f1801a6d',
        'scripts/stability-gates/process.mjs': '12391db1cca0420525756ad4c601b5f4ac3354ee763e077298ebfcbe827a9cfb',
        'scripts/stability-gates/input.mjs': '09e5212dc41e61536461a8e1a13655463427db974ad59e82944b0d01efe80da0',
        'scripts/stability-gates/release-build-input.mjs': '899460125e7a20f7f3ed044e6484c4fec7a7ad904311b971d998bfab5b0444e0',
        'scripts/stability-gates/interruption.mjs': '9e4c2e9c5c7a4f2df480593bd0f7ffa2b863f610e756e618bbdda4743064bbc8',
        'scripts/stability-gates/linux-tree.mjs': '555c1d5d1b22b995952e4458d3be6cf09a44cf3b2492ddcaf8cafa20a1a772f3'}
_COMPLETIONS = weakref.WeakKeyDictionary()


class _CompletedNotAffected:
    __slots__ = ("__weakref__",)

    def __new__(cls):
        raise TypeError("only the current owning execution creates a disposition")


def not_affected_record(value):
    require(type(value) is _CompletedNotAffected and value in _COMPLETIONS,
            "only same-invocation actual block-buffer owner disposition")
    return decode(_COMPLETIONS[value])


def _projection(pin):
    return {key: pin[key] for key in ("path", "bytes", "sha256")}


def _expected(material):
    return {"path": material["path"], "bytes": len(material["bytes"]), "sha256": material["sha256"]}


def validate_test_output(raw, stderr):
    require(isinstance(raw, bytes) and len(raw) <= 1024 * 1024 and stderr == b"", "complete bounded owner streams")
    text = raw.decode("utf-8").replace("\r\n", "\n")
    require(re.search(r"[\x00-\x08\x0b-\x1f\x7f-\x9f]", text) is None, "plain owner transcript")
    expected = "\nrunning 3 tests\n" + "".join("test " + name + " ... ok\n" for name in TESTS)
    require(text.startswith(expected), "exact three complete regression names once")
    require(re.fullmatch(re.escape(expected) + r"\ntest result: ok\. 3 passed; 0 failed; 0 ignored; 0 measured; "
                        r"0 filtered out; finished in [0-9]+\.[0-9]+s\n\n", text) is not None,
            "no skipped, failed, extra or filtered owner proof")


def _call(node, root, output, cargo, rustc):
    directory = output / "block-buffer-owner"
    argv = [str(node), str(root / "scripts/stability-gates/dependency-block-buffer-owner.mjs"),
            str(root), str(directory), str(cargo), str(rustc)]
    retain(output, "block-buffer-bridge.command.json", encode({"argv": argv, "timeoutSeconds": 120}))
    allowed = ("PATH", "HOME", "USERPROFILE", "TMP", "TEMP", "TMPDIR", "SystemRoot", "WINDIR",
               "CARGO_HOME", "RUSTUP_HOME", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "http_proxy",
               "https_proxy", "no_proxy", "SSL_CERT_FILE", "SSL_CERT_DIR", "NODE_EXTRA_CA_CERTS", "NODE_USE_SYSTEM_CA",
               "INCLUDE", "LIB", "LIBPATH", "WindowsSdkDir", "WindowsSDKVersion", "UniversalCRTSdkDir", "UCRTVersion",
               "VCToolsInstallDir", "VCToolsVersion", "ZRYNA_DEPENDENCY_LINKER")
    environment = {key: os.environ[key] for key in allowed if key in os.environ}
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        code, error = None, None
        try:
            code = subprocess.run(argv, cwd=root, env=environment, stdin=subprocess.DEVNULL,
                                  stdout=stdout, stderr=stderr, timeout=120, check=False).returncode
        except subprocess.TimeoutExpired:
            error = "new fixed block-buffer bridge exceeded its 120-second outer guard"
        stdout.seek(0); raw = stdout.read(1024 * 1024 + 1)
        stderr.seek(0); diagnostic = stderr.read(1024 * 1024 + 1)
    retain(output, "block-buffer-bridge.stdout.log", raw)
    retain(output, "block-buffer-bridge.stderr.log", diagnostic)
    retain(output, "block-buffer-bridge.status.json", encode({"exitCode": code, "error": error,
        "stdoutBytes": len(raw), "stderrBytes": len(diagnostic), "stdoutSha256": sha(raw), "stderrSha256": sha(diagnostic)}))
    require(len(raw) <= 1024 * 1024 and len(diagnostic) <= 1024 * 1024, "bounded complete bridge acquisition")
    require(code == 0 and error is None and not diagnostic, "actual fixed block-buffer owner must complete")
    return decode(raw, 1024 * 1024)


def validate_result(value, root, output, tools, sources):
    require(set(value) == {"format", "status", "root", "output", "host", "package", "environment", "guards",
            "inputs", "proofSources", "tools", "registry", "cargoConfiguration", "graph", "artifacts", "commands",
            "tests", "error", "sourceAuthenticated", "hostAuthenticated", "fullSecurityQualified", "advisoryDispositionIssued"},
            "closed scoped owner result")
    require(value["format"] == "zryna.block-buffer-owner.v1" and value["status"] == "passed" and value["error"] is None,
            "current complete block-buffer owner")
    require(value["root"] == str(root) and value["output"] == str(output), "same owner source/output invocation")
    for key in ("sourceAuthenticated", "hostAuthenticated", "fullSecurityQualified", "advisoryDispositionIssued"):
        require(value[key] is False, "scoped owner cannot issue broader authority")
    require(value["package"] == {"name": "block-buffer", "version": "0.10.4", "source": SOURCE, "checksum": CHECKSUM},
            "one exact original package")
    require(value["tests"] == TESTS, "fixed regression corpus")
    for key in ("inputs", "proofSources", "tools", "registry", "cargoConfiguration"):
        require(value[key]["before"] == value[key]["after"], "stable before/after " + key)
    require(value["host"]["arch"] == "x64" and (value["host"]["platform"], value["host"]["target"]) in
            (("linux", "x86_64-unknown-linux-gnu"), ("win32", "x86_64-pc-windows-msvc")), "supported scoped owner host")
    for key, name in (("manifest", "Cargo.toml"), ("lock", "Cargo.lock")):
        require(_projection(value["inputs"]["before"][key]) ==
                {"path": str(root / name), "bytes": ROOT_PINS[name][0], "sha256": ROOT_PINS[name][1]}, "frozen Cargo input")
    require({pin["path"]: _projection(pin) for pin in value["proofSources"]["before"]} ==
            {pin["path"]: _expected(pin) for pin in sources}, "every reviewed owning source")
    require(len(value["proofSources"]["before"]) == len(sources), "unique owning source pins")
    expected_tools = {"cargo", "rustc", "node"} | ({"linker"} if value["host"]["platform"] == "win32" else set())
    require(set(tools) == set(value["tools"]["before"]) == expected_tools, "exact supported-platform admitted tools")
    for key in expected_tools:
        require(_projection(value["tools"]["before"][key]) == _expected(tools[key]), "actual admitted owner tool " + key)
    if "linker" in expected_tools:
        linker = tools["linker"]["path"]
        require(value["environment"].get("ZRYNA_DEPENDENCY_LINKER") == linker and
                value["environment"].get("CARGO_ENCODED_RUSTFLAGS") == "-C\x1flinker=" + linker,
                "fixed admitted Cargo linker binding")
        require(value["commands"]["compile"]["command"]["args"][-2:] == ["-C", "linker=" + linker],
                "fixed admitted direct probe compiler linker binding")
    for key, name, version in (("sha2", "sha2", "0.10.9"), ("digest", "digest", "0.10.7"),
                               ("blockBuffer", "block-buffer", "0.10.4")):
        package = value["graph"][key]
        require(package["name"] == name and package["version"] == version and package["source"] == SOURCE,
                "exact locked selected dependency graph")
    directory = Path(value["graph"]["blockBuffer"]["manifestPath"]).parent
    expected = {str(directory / name): {"path": str(directory / name), "bytes": size, "sha256": digest}
                for name, (size, digest) in REGISTRY_PINS.items()}
    require(len(value["registry"]["before"]) == 3 and
            {pin["path"]: _projection(pin) for pin in value["registry"]["before"]} == expected,
            "exact reviewed separate-cursor original sources")
    require(value["guards"] == {"cargoVersion": 3000, "rustcVersion": 3000, "metadata": 12000,
            "build": 40000, "compile": 15000, "probe": 5000}, "fixed new owner guards")
    require(set(value["commands"]) == {"cargoVersion", "rustcVersion", "metadata", "build", "compile", "probe"},
            "all fixed owner commands")
    for name, command in value["commands"].items():
        termination = command["termination"]
        require(termination["exitCode"] == 0 and termination["signal"] is None and termination["error"] is None,
                "actual complete owner command " + name)
    require(value["commands"]["probe"]["command"] == {"executable": value["artifacts"]["binary"]["path"],
            "args": ["--test-threads=1", "--color", "never"], "cwd": str(root), "timeoutMs": 5000},
            "actual fixed probe invocation")
    raw = snapshot(output / "block-buffer-probe.stdout.log", 1024 * 1024)["bytes"]
    diagnostic = snapshot(output / "block-buffer-probe.stderr.log", 1024 * 1024)["bytes"]
    validate_test_output(raw, diagnostic)
    for stream, data in (("stdout", raw), ("stderr", diagnostic)):
        require(value["commands"]["probe"]["logs"][stream] ==
                {"path": str(output / ("block-buffer-probe." + stream + ".log")), "bytes": len(data), "sha256": sha(data)},
                "actual owner raw log binding")
    for key in ("sha2", "binary"):
        pin = value["artifacts"][key]; path = Path(pin["path"])
        require(output in path.parents and str(path) == str(path.resolve()), "fresh outside-source owner artifact")
        require(_projection(pin) == _expected(snapshot(path, 256 * 1024 * 1024)), "actual owner artifact " + key)


def qualify_not_affected(node, root, output, cargo, rustc):
    captured = []
    for name, expected in PINS.items():
        material = snapshot(root / name)
        require(material["sha256"] == expected, "reviewed fixed owner source: " + name)
        captured.append(material)
    inputs = [snapshot(root / name) for name in ROOT_PINS]
    for name, material in zip(ROOT_PINS, inputs):
        require((len(material["bytes"]), material["sha256"]) == ROOT_PINS[name], "unchanged reviewed Cargo graph")
    lock = tomllib.loads(inputs[1]["bytes"].decode("utf-8"))
    selected = [row for row in lock["package"] if row["name"] == "block-buffer"]
    require(len(selected) == 1 and selected[0]["version"] == "0.10.4" and selected[0]["source"] == SOURCE and
            selected[0]["checksum"] == CHECKSUM, "exact locked original package archive")
    tools = {name: snapshot(path, 256 * 1024 * 1024) for name, path in (("node", node), ("cargo", cargo), ("rustc", rustc))}
    if os.environ.get("ZRYNA_DEPENDENCY_LINKER"):
        tools["linker"] = snapshot(Path(os.environ["ZRYNA_DEPENDENCY_LINKER"]), 256 * 1024 * 1024)
    value = _call(node, root, output, cargo, rustc)
    validate_result(value, root, output / "block-buffer-owner", tools, captured)
    for material in [*captured, *inputs, *tools.values()]:
        unchanged(material)
    token = object.__new__(_CompletedNotAffected)
    _COMPLETIONS[token] = encode({"advisory": ADVISORY, "advisorySha256": DETAIL_SHA, "query": QUERY,
        "packageArchiveSha256": CHECKSUM, "disposition": "exact-reviewed-original-source-not-affected-by-inline-cursor-advisory",
        "owner": value, "sourceAuthenticated": False, "scope": "Only this mechanism on exact original 0.10.4; no Miri or whole-security claim"})
    return token
