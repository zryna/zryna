"""Synthetic owner receipts/files for parser and capability tests; never execute tools."""
import copy
from pathlib import Path
import tempfile
import test_audit
import dependency_audit_block_buffer as owner
from dependency_audit_inputs import encode, sha, snapshot

DETAIL = test_audit.decode((test_audit.FIXTURES / "block-buffer-advisory.json").read_bytes())
MATCH = [{"query": owner.QUERY, "advisories": [{"id": owner.ADVISORY, "modified": "2026-08-19T19:30:06.686866Z"}]}]
RAW = ("\nrunning 3 tests\n" + "".join("test " + name + " ... ok\n" for name in owner.TESTS) +
       "\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n").encode()


class Fixture:
    def __init__(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name).resolve()
        self.root = self.base / "source"; self.root.mkdir()
        self.audit_output = self.base / "output"; self.audit_output.mkdir()
        self.output = self.audit_output / "block-buffer-owner"; self.output.mkdir()
        for name in owner.ROOT_PINS:
            (self.root / name).write_bytes((test_audit.BASE / name).read_bytes())
        self.source = self.root / "synthetic-fixed-owner.mjs"; self.source.write_bytes(b"synthetic source; never executed\n")
        self.sources = [snapshot(self.source)]
        self.pins = {self.source.name: sha(self.source.read_bytes())}
        self.tools = {}
        for name in ["node", "cargo", "rustc"]:
            path = self.base / name; path.write_bytes(b"synthetic tool; never executed\n")
            self.tools[name] = snapshot(path)
        self.receipt = self.value()

    @staticmethod
    def project(pin):
        return {"path": pin["path"], "bytes": len(pin["bytes"]), "sha256": pin["sha256"]}

    def value(self):
        def pair(value): return {"before": copy.deepcopy(value), "after": copy.deepcopy(value)}
        inputs = {key: {"path": str(self.root / name), "bytes": size, "sha256": digest}
                  for key, (name, (size, digest)) in zip(["manifest", "lock"], owner.ROOT_PINS.items())}
        registry_root = self.base / "synthetic-registry" / "block-buffer-0.10.4"
        registry = [{"path": str(registry_root / name), "bytes": size, "sha256": digest}
                    for name, (size, digest) in owner.REGISTRY_PINS.items()]
        graph = {key: {"name": name, "version": version, "source": owner.SOURCE,
                 "manifestPath": str(registry_root / "Cargo.toml")}
                 for key, name, version in [("sha2", "sha2", "0.10.9"), ("digest", "digest", "0.10.7"),
                                             ("blockBuffer", "block-buffer", "0.10.4")]}
        artifacts = {}
        for name in ["sha2", "binary"]:
            path = self.output / name; path.write_bytes(b"synthetic artifact; never executed\n")
            artifacts[name] = self.project(snapshot(path))
        for stream, raw in [("stdout", RAW), ("stderr", b"")]:
            (self.output / ("block-buffer-probe." + stream + ".log")).write_bytes(raw)
        commands = {name: {"command": {}, "termination": {"exitCode": 0, "signal": None, "error": None, "elapsedMs": 1},
                          "logs": {}} for name in ["cargoVersion", "rustcVersion", "metadata", "build", "compile", "probe"]}
        commands["probe"]["command"] = {"executable": artifacts["binary"]["path"],
            "args": ["--test-threads=1", "--color", "never"], "cwd": str(self.root), "timeoutMs": 5000}
        commands["probe"]["logs"] = {stream: {"path": str(self.output / ("block-buffer-probe." + stream + ".log")),
            "bytes": len(raw), "sha256": sha(raw)} for stream, raw in [("stdout", RAW), ("stderr", b"")]}
        return {"format": "zryna.block-buffer-owner.v1", "status": "passed", "root": str(self.root), "output": str(self.output),
            "host": {"platform": "linux", "arch": "x64", "target": "x86_64-unknown-linux-gnu"},
            "package": {"name": "block-buffer", "version": "0.10.4", "source": owner.SOURCE, "checksum": owner.CHECKSUM},
            "environment": {}, "guards": {"cargoVersion": 3000, "rustcVersion": 3000, "metadata": 12000,
                 "build": 40000, "compile": 15000, "probe": 5000}, "inputs": pair(inputs),
            "proofSources": pair([self.project(pin) for pin in self.sources]),
            "tools": pair({name: self.project(pin) for name, pin in self.tools.items()}),
            "registry": pair(registry), "cargoConfiguration": pair([]), "graph": graph, "artifacts": artifacts,
            "commands": commands, "tests": owner.TESTS.copy(), "error": None, "sourceAuthenticated": False,
            "hostAuthenticated": False, "fullSecurityQualified": False, "advisoryDispositionIssued": False}

    def validate(self):
        owner.validate_result(self.receipt, self.root, self.output, self.tools, self.sources)

    def close(self):
        self.temporary.cleanup()
