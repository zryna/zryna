#!/usr/bin/env python3
"""Exact lock advisory observation CLI. Never an authenticated #418 security/closure issuer."""
import argparse
import datetime
import os
from pathlib import Path
import re
import sys
from dependency_audit_inputs import (cargo_inputs, decode, encode, public_pin, require,
    retain, sha, snapshot, unchanged, VERSION)
from dependency_audit_msal import qualify_backport
from dependency_audit_node import node_call
from dependency_audit_osv import ADVISORY, Capture, bind_modified, classify, parse_advisory, parse_batch


def queries_for(cargo, node):
    require(isinstance(node, dict) and node.get("format") == "zryna.dependency-lock-inputs.v1", "Node lock reader shape")
    require(node.get("node") == "v22.22.1" and node.get("yamlVersion") == "2.8.3", "actual fixed parser tools")
    require(isinstance(node.get("packages"), list), "npm registry inventory")
    unique = {}
    for item in cargo + node["packages"]:
        require(isinstance(item, dict) and set(item) == {"package", "version"}, "exact query input")
        package, version = item["package"], item["version"]
        require(isinstance(package, dict) and set(package) == {"ecosystem", "name"}, "exact package identity")
        require(package["ecosystem"] in ("crates.io", "npm") and isinstance(package["name"], str), "registry ecosystem")
        pattern = r"[A-Za-z0-9_-]+" if package["ecosystem"] == "crates.io" else r"(?:@[A-Za-z0-9_.-]+/)?[A-Za-z0-9_.-]+"
        require(len(package["name"]) <= 256 and re.fullmatch(pattern, package["name"]), "ordinary registry package name")
        require(isinstance(version, str) and VERSION.fullmatch(version), "exact registry version")
        key = (package["ecosystem"], package["name"], version)
        require(key not in unique, "duplicate registry query")
        unique[key] = item
    require(0 < len(unique) <= 4096, "finite complete registry inventory")
    return [unique[key] for key in sorted(unique)]


def collect(node, root, output):
    tracked = [snapshot(root / path) for path in ("Cargo.lock", "pnpm-lock.yaml", "pnpm-workspace.yaml", "package.json")]
    for material in tracked:
        retain(output, "input-" + Path(material["path"]).name, material["bytes"])
    cargo, local, manifests = cargo_inputs(root, tracked[0]["bytes"])
    tracked.extend(manifests)
    tools = [snapshot(node, 256 * 1024 * 1024), snapshot(Path(sys.executable).resolve(), 256 * 1024 * 1024)]
    tool_versions = {"python": sys.version, "pythonExecutable": str(Path(sys.executable).resolve()),
                     "nodeExecutable": str(node), "pnpm": "not executed; lock generator is a declaration only"}
    own = Path(__file__).parent
    tracked.extend(snapshot(own / path) for path in ("dependency_audit.py", "dependency_audit_inputs.py",
        "dependency_audit_node.py", "dependency_audit_osv.py", "dependency_audit_msal.py",
        "dependency-lock-inputs.mjs", "dependency-lock-parser.mjs"))
    parsed = node_call(node, root, output, "--locks", "node-lock-inputs")
    queries = queries_for(cargo, parsed)
    tool_versions.update({"node": parsed["node"], "yaml": parsed["yamlVersion"],
                          "yamlEntrySha256": parsed["yamlEntrySha256"]})
    for material in tracked + tools:
        unchanged(material)
    freeze = {"format": "zryna.dependency-advisory-before-query.v1", "at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "root": str(root), "lockPins": [public_pin(row) for row in tracked[:4]],
        "selectedInputPins": [public_pin(row) for row in tracked[4:]], "toolPins": [public_pin(row) for row in tools],
        "toolVersions": tool_versions, "queries": queries, "queryCount": len(queries),
        "localNotRegistryAudited": local, "endpoint": "https://api.osv.dev/v1/", "batchMaximum": 100,
        "proxyAndCA": "ordinary urllib environment/default trust preserved; no secrets copied into evidence",
        "sourceAuthenticated": False, "proofQualified": False, "candidateSecurityQualified": False,
        "closureStatus": "blocked", "thresholds": None}
    retain(output, "PRE_QUERY_FREEZE.json", encode(freeze))
    capture, results = Capture(output), []
    for offset in range(0, len(queries), 100):
        batch = queries[offset:offset + 100]
        raw = capture.request("querybatch", f"batch-{offset:04d}", encode({"queries": batch}))
        results.extend(parse_batch(raw, batch))
        for material in tracked + tools:
            unchanged(material)
    identifiers = sorted({item["id"] for row in results for item in row["advisories"]})
    require(len(identifiers) <= 256, "finite full advisory detail inventory")
    details = {}
    for identifier in identifiers:
        raw = capture.request("vulns/" + identifier, identifier, maximum=4 * 1024 * 1024)
        details[identifier] = parse_advisory(raw, identifier)
    # Retain every raw match before any source-bound disposition or SDK execution.
    retain(output, "RAW_ADVISORY_INDEX.json", encode({"results": results, "advisoryIds": identifiers}))
    bind_modified(results, details)  # Every fetched pair, BEFORE any SDK owner or disposition.
    msal = any(row["query"] == {"package": {"ecosystem": "npm", "name": "@azure/msal-node"},
        "version": "6.0.1"} and any(item["id"] == ADVISORY for item in row["advisories"]) for row in results)
    completed = qualify_backport(node, root, output) if msal else None
    disposition = classify(results, details, completed)
    for material in tracked + tools:
        unchanged(material)
    report = {"format": "zryna.observed-dependency-gate.v1", **disposition,
        "lockPins": freeze["lockPins"], "toolVersions": tool_versions, "queries": queries,
        "results": results, "advisoryIds": identifiers, "localNotRegistryAudited": local,
        "sourceAuthenticated": False, "proofQualified": False, "candidateSecurityQualified": False,
        "closureStatus": "blocked", "thresholds": None,
        "scope": "exact lock registry advisory snapshot and one exact local upstream-backport disposition only",
        "exclusions": ["OS/SDK/native-provider advisories", "first-party security review", "whole #418 security/closure"]}
    retain(output, "REPORT.json", encode(report))
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True)
    parser.add_argument("--node", required=True, help="actual trusted operator-admitted Node22.22.1 executable")
    parser.add_argument("--output", required=True, help="fresh private evidence directory OUTSIDE the checkout")
    options = parser.parse_args(argv)
    root, node, output = (Path(value) for value in (options.root, options.node, options.output))
    require(all(path.is_absolute() and str(path) == str(path.resolve()) for path in (root, node, output)), "canonical absolute paths")
    require(root.is_dir() and output.parent.is_dir() and not output.parent.is_symlink(), "trusted existing ancestors")
    require(root not in output.parents and output not in root.parents and output != root, "output outside source")
    output.mkdir(mode=0o700)  # Exclusive; never overwrite previous captures.
    try:
        report = collect(node, root, output)
    except Exception as error:
        retain(output, "INVALID.json", encode({"format": "zryna.observed-dependency-gate.v1",
            "observedDependencyGate": "invalid", "errorType": type(error).__name__, "error": str(error)[:4096],
            "sourceAuthenticated": False, "proofQualified": False, "candidateSecurityQualified": False,
            "closureStatus": "blocked", "thresholds": None}))
        print("dependency advisory capture invalid; retain output", file=sys.stderr)
        return 2
    print(encode({"observedDependencyGate": report["observedDependencyGate"],
        "rawAdvisoryMatches": len(report["rawAdvisoryMatches"]),
        "sourceBoundRemediations": len(report["sourceBoundRemediations"]),
        "candidateSecurityQualified": False, "closureStatus": "blocked"}).decode(), end="")
    return 0 if report["observedDependencyGate"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
