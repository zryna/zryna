"""Synthetic parser/fake-IO controls only. No network, Node, Cargo, SDK or guest execution."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

BASE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(BASE / "scripts/stability-gates"))
from dependency_audit import queries_for
from dependency_audit_inputs import AuditError, cargo_inputs, decode, encode, retain, sha, snapshot, unchanged
import dependency_audit_msal as msal
from dependency_audit_osv import ADVISORY, Capture, NoRedirect, classify, parse_advisory, parse_batch

FIXTURES = Path(__file__).with_name("fixtures")
DETAIL = decode((FIXTURES / "msal-advisory.json").read_bytes())
NAMES = decode((FIXTURES / "sdk-names.json").read_bytes())
QUERY = {"package": {"ecosystem": "npm", "name": "@azure/msal-node"}, "version": "6.0.1"}
MATCH = [{"query": QUERY, "advisories": [{"id": ADVISORY, "modified": DETAIL["modified"]}]}]
TAP = (FIXTURES / "synthetic-owner.tap").read_bytes()


def owner_status(raw=TAP):
    return {"format": "zryna.dependency-msal-owner-status.v1", "argv": ["/synthetic/node", "--test",
        "--test-reporter=tap", "tests/msal-backport.test.mjs"], "exitCode": 0, "signal": None,
        "error": None, "stdoutBytes": len(raw), "stderrBytes": 0, "stdoutSha256": sha(raw), "stderrSha256": sha(b"")}


class FakeResponse:
    def __init__(self, raw, status=200):
        self.raw, self.status, self.headers = raw, status, {"Date": "synthetic-only"}
    def __enter__(self): return self
    def __exit__(self, *_args): return None
    def read1(self, count):
        result, self.raw = self.raw[:count], self.raw[count:]
        return result


class AuditControls(unittest.TestCase):
    def test_json_duplicate_utf8_nonfinite_and_depth_reject(self):
        for raw in [b'{"a":1,"a":2}', b'\xff', b'{"a":NaN}', b'{"a":1e999}', b'[' * 34 + b'0' + b']' * 34]:
            with self.assertRaises((AuditError, ValueError)): decode(raw)

    def cargo(self, root, source='source = "registry+https://github.com/rust-lang/crates.io-index"', extra='', checksum='a' * 64):
        (root / "Cargo.toml").write_text('[workspace]\nmembers=["local/member"]\n[workspace.package]\nversion="1.0.0"\n')
        (root / "local/member").mkdir(parents=True)
        (root / "local/member/Cargo.toml").write_text('[package]\nname="ours"\nversion.workspace=true\n')
        raw = ('version=4\n[[package]]\nname="ours"\nversion="1.0.0"\n[[package]]\nname="registry"\nversion="1.2.3"\n'
               + source + '\nchecksum="' + checksum + '"\n' + extra).encode()
        return cargo_inputs(root, raw)

    def test_registered_local_identity_is_explicit_not_registry_audit(self):
        with tempfile.TemporaryDirectory() as tmp:
            packages, local, materials = self.cargo(Path(tmp))
            self.assertEqual(packages[0]["package"]["ecosystem"], "crates.io")
            self.assertEqual(local[0]["name"], "ours"); self.assertEqual(len(materials), 2)

    def test_git_unknown_local_and_invalid_checksum_reject(self):
        for source in ['source="git+https://example.invalid/repo"', '', 'source="registry+https://other.invalid"']:
            with tempfile.TemporaryDirectory() as tmp, self.assertRaises(AuditError): self.cargo(Path(tmp), source)
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(AuditError): self.cargo(Path(tmp), checksum='invalid')

    def test_duplicate_cargo_package_rejects(self):
        extra = '\n[[package]]\nname="ours"\nversion="1.0.0"\n'
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(AuditError): self.cargo(Path(tmp), extra=extra)

    def test_full_registry_queries_and_unknown_name_version_reject(self):
        node = {"format": "zryna.dependency-lock-inputs.v1", "node": "v22.22.1", "yamlVersion": "2.8.3", "packages": [QUERY]}
        self.assertEqual(queries_for([], node), [QUERY])
        for mutate in [lambda n: n["packages"].append(QUERY), lambda n: n.update(node="v24.0.0"),
                       lambda n: n["packages"][0].update(version="git:main"),
                       lambda n: n["packages"][0]["package"].update(name="../unknown")]:
            bad = copy.deepcopy(node); mutate(bad)
            with self.assertRaises(AuditError): queries_for([], bad)

    def test_batch_complete_results_preserve_every_match(self):
        self.assertEqual(parse_batch(encode({"results": [{"vulns": MATCH[0]["advisories"]}, {}]}), [QUERY, QUERY])[0], MATCH[0])

    def test_batch_missing_unknown_paginated_duplicate_and_null_reject(self):
        for row in [{}, {"results": []}, {"results": [{"vulns": None}]}, {"results": [{}], "ok": True},
                    {"results": [{"next_page_token": "more"}]},
                    {"results": [{"vulns": MATCH[0]["advisories"] * 2}]}]:
            with self.assertRaises(AuditError): parse_batch(encode(row), [QUERY])

    def test_withdrawn_or_wrong_advisory_id_is_never_silently_ignored(self):
        self.assertEqual(parse_advisory(encode(DETAIL), ADVISORY), DETAIL)
        for change in [{"withdrawn": "2026-01-01T00:00:00Z"}, {"id": "OTHER"}, {"affected": []}]:
            with self.assertRaises(AuditError): parse_advisory(encode({**DETAIL, **change}), ADVISORY)

    def test_raw_msal_without_actual_owner_remains_failed_and_visible(self):
        result = classify(MATCH, {ADVISORY: DETAIL}, None)
        self.assertEqual(result["observedDependencyGate"], "fail"); self.assertEqual(len(result["rawAdvisoryMatches"]), 1)

    def test_known_msal_advisory_cannot_disappear(self):
        with self.assertRaises(AuditError): classify([{**MATCH[0], "advisories": []}], {}, None)

    def test_decoded_pass_json_cannot_be_backport_disposition(self):
        with self.assertRaises(AuditError): classify(MATCH, {ADVISORY: DETAIL}, {"pass": True})
        with self.assertRaises(AuditError): msal._CompletedBackport({}, object())

    def test_exact_twelve_case_owner_and_crlf_positive(self):
        msal.validate_owner(TAP, owner_status(), NAMES)
        raw = TAP.replace(b'\n', b'\r\n'); msal.validate_owner(raw, owner_status(raw), NAMES)

    def test_owner_missing_duplicate_skip_fail_control_and_status_reject(self):
        for raw in [TAP.replace(b'# tests 12', b'# tests 0'), TAP.replace(b'# skipped 0', b'# skipped 1'),
                    TAP.replace(b'    ok 1 - ', b'    not ok 1 - ', 1), TAP + b'\x1b[31m',
                    TAP.replace(NAMES[0].encode(), b'short-name'), TAP + b'    ok 1 - extra\n']:
            with self.assertRaises(AuditError): msal.validate_owner(raw, owner_status(raw), NAMES)
        status = owner_status(); status['exitCode'] = 1
        with self.assertRaises(AuditError): msal.validate_owner(TAP, status, NAMES)

    def test_http_raw_saved_before_bad_json_validation(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp); cap = Capture(output)
            cap.opener.open = lambda *_a, **_k: FakeResponse(b'{invalid')
            raw = cap.request('querybatch', 'bad', b'{}')
            self.assertEqual((output / 'bad.response.json').read_bytes(), raw)
            with self.assertRaises(AuditError): parse_batch(raw, [QUERY])

    def test_http_failure_first_extra_and_redirect_fail_closed(self):
        for raw, status in [(b'HTTP failure', 503), (b'x' * 17, 200)]:
            with tempfile.TemporaryDirectory() as tmp, self.assertRaises(AuditError):
                cap = Capture(Path(tmp)); cap.opener.open = lambda *_a, **_k: FakeResponse(raw, status)
                try: cap.request('querybatch', 'failure', b'{}', maximum=16)
                finally: self.assertTrue((Path(tmp) / 'failure.response.json').exists())
        self.assertIsNone(NoRedirect().redirect_request(None))
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(AuditError):
            cap = Capture(Path(tmp)); response = FakeResponse(b'prefix'); calls = []
            def interrupted(count):
                if calls: raise TimeoutError('synthetic read timeout')
                calls.append(1); return b'prefix'
            response.read1 = interrupted; cap.opener.open = lambda *_a, **_k: response
            try: cap.request('querybatch', 'partial', b'{}')
            finally: self.assertEqual((Path(tmp) / 'partial.response.json').read_bytes(), b'prefix')

    def test_stable_input_and_create_only_retention(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); path = root / 'input'; path.write_bytes(b'original'); first = snapshot(path)
            unchanged(first); path.write_bytes(b'changed')
            with self.assertRaises(AuditError): unchanged(first)
            retain(root, 'once.json', b'{}')
            with self.assertRaises(FileExistsError): retain(root, 'once.json', b'changed')

    def test_synthetic_same_invocation_owner_can_only_remediate_exact_frozen_finding(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / 'tests').mkdir(); prov = root / 'tests/provenance.json'
            prov.write_bytes(encode({'expectedCaseNames': NAMES, 'upstream': {'commit': msal.UPSTREAM}}))
            before = {'format': 'zryna.dependency-msal-materials.v1', 'installed': [{}] * 7}
            def fake(_node, _root, output, mode, _name):
                if mode == '--msal-control':
                    (output / 'msal-owner.stdout.log').write_bytes(TAP); (output / 'msal-owner.stderr.log').write_bytes(b'')
                    return owner_status()
                return before
            with patch.object(msal, 'PINS', {'tests/provenance.json': sha(prov.read_bytes())}), patch.object(msal, 'node_call', fake):
                completed = msal.qualify_backport(Path('/synthetic/node'), root, root)
            self.assertEqual(classify(MATCH, {ADVISORY: DETAIL}, completed)['observedDependencyGate'], 'pass')
            altered = {**DETAIL, 'summary': 'new or changed advisory context'}
            self.assertEqual(classify(MATCH, {ADVISORY: altered}, completed)['observedDependencyGate'], 'fail')
            other = copy.deepcopy(MATCH); other[0]['advisories'].append({'id': 'NEW', 'modified': 'new'})
            self.assertEqual(classify(other, {ADVISORY: DETAIL, 'NEW': {'id': 'NEW'}}, completed)['observedDependencyGate'], 'fail')

    def test_no_matches_in_complete_queried_inventory_has_no_closure_authority(self):
        result = classify([{'query': {'package': {'ecosystem': 'npm', 'name': 'ajv'}, 'version': '8.18.0'},
                            'advisories': []}], {}, None)
        self.assertEqual(result['observedDependencyGate'], 'pass')
        self.assertEqual(result['sourceBoundRemediations'], [])


if __name__ == '__main__':
    unittest.main()
