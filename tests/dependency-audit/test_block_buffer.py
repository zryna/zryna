"""Synthetic fail-closed controls; these tests never execute Cargo, rustc or an SDK."""
import copy
import unittest
from unittest.mock import patch
import block_buffer_fixture as fixture
import dependency_audit as gate
import dependency_audit_block_buffer as owner
from dependency_audit_inputs import AuditError, encode, sha
from dependency_audit_osv import bind_modified, classify


class BlockBufferControls(unittest.TestCase):
    def fresh(self):
        value = fixture.Fixture(); self.addCleanup(value.close); return value

    def complete(self, value):
        with patch.object(owner, "PINS", value.pins), patch.object(owner, "_call", return_value=value.receipt):
            return owner.qualify_not_affected(value.tools["node"]["path"], value.root, value.audit_output,
                value.tools["cargo"]["path"], value.tools["rustc"]["path"])

    def test_exact_retained_pair_alone_stays_failed_and_raw_match_visible(self):
        self.assertEqual(sha(encode(fixture.DETAIL)), owner.DETAIL_SHA)
        bind_modified(fixture.MATCH, {owner.ADVISORY: fixture.DETAIL})
        result = classify(fixture.MATCH, {owner.ADVISORY: fixture.DETAIL}, None)
        self.assertEqual(result["observedDependencyGate"], "fail")
        self.assertEqual(result["rawAdvisoryMatches"][0]["advisory"], owner.ADVISORY)
        self.assertEqual(result["sourceBoundNotAffected"], [])

    def test_pair_is_not_generic_rounding_or_other_query_or_modified_body(self):
        for mutate in [lambda rows: rows[0]["advisories"][0].update(modified="2026-08-19T19:30:06.686867Z"),
                       lambda rows: rows[0]["query"].update(version="0.12.0"),
                       lambda rows: rows[0]["query"]["package"].update(name="block_buffer")]:
            bad = copy.deepcopy(fixture.MATCH); mutate(bad)
            with self.assertRaises(AuditError): bind_modified(bad, {owner.ADVISORY: fixture.DETAIL})
        for change in [{"summary": "changed context"}, {"modified": "2026-08-19T19:30:06.686866978Z"}]:
            with self.assertRaises(AuditError): bind_modified(fixture.MATCH, {owner.ADVISORY: {**fixture.DETAIL, **change}})

    def test_same_invocation_completion_retains_raw_match_and_separate_disposition(self):
        value = self.fresh(); value.validate(); complete = self.complete(value)
        result = classify(fixture.MATCH, {owner.ADVISORY: fixture.DETAIL}, None, complete)
        self.assertEqual(result["observedDependencyGate"], "pass")
        self.assertEqual(len(result["rawAdvisoryMatches"]), 1)
        self.assertEqual(len(result["sourceBoundNotAffected"]), 1)
        self.assertEqual(result["sourceBoundRemediations"], [])
        record = owner.not_affected_record(complete); record["query"]["version"] = "changed"
        self.assertEqual(owner.not_affected_record(complete)["query"], owner.QUERY)

    def test_json_copy_and_constructor_cannot_mint_or_relabel_completion(self):
        value = self.fresh(); complete = self.complete(value)
        for decoded in [value.receipt, owner.not_affected_record(complete), {"status": "passed"}]:
            with self.assertRaises(AuditError): classify(fixture.MATCH, {owner.ADVISORY: fixture.DETAIL}, None, decoded)
        with self.assertRaises(TypeError): owner._CompletedNotAffected()
        with self.assertRaises(TypeError): copy.copy(complete)

    def test_owner_cannot_dispose_another_version_advisory_or_changed_full_detail(self):
        value = self.fresh(); complete = self.complete(value)
        changed = {**fixture.DETAIL, "summary": "changed"}
        self.assertEqual(classify(fixture.MATCH, {owner.ADVISORY: changed}, None, complete)["observedDependencyGate"], "fail")
        rows = copy.deepcopy(fixture.MATCH); rows[0]["query"]["version"] = "0.12.0"
        self.assertEqual(classify(rows, {owner.ADVISORY: fixture.DETAIL}, None, complete)["observedDependencyGate"], "fail")
        rows = copy.deepcopy(fixture.MATCH); rows[0]["advisories"].append({"id": "NEW", "modified": "new"})
        result = classify(rows, {owner.ADVISORY: fixture.DETAIL, "NEW": {"id": "NEW"}}, None, complete)
        self.assertEqual(result["observedDependencyGate"], "fail"); self.assertEqual(len(result["rawAdvisoryMatches"]), 2)

    def test_receipt_status_authority_host_tools_and_all_materials_are_bound(self):
        mutations = [lambda x: x.update(status="failed"), lambda x: x.update(error="cleanup"),
            lambda x: x.update(sourceAuthenticated=True), lambda x: x.update(hostAuthenticated=True),
            lambda x: x.update(fullSecurityQualified=True), lambda x: x.update(advisoryDispositionIssued=True),
            lambda x: x["host"].update(target="other"), lambda x: x["package"].update(checksum="0" * 64),
            lambda x: x["guards"].update(build=40001), lambda x: x["tools"]["after"]["rustc"].update(bytes=1),
            lambda x: x["registry"]["before"][0].update(sha256="0" * 64),
            lambda x: x["proofSources"]["after"].clear(), lambda x: x["inputs"]["after"]["lock"].update(bytes=1),
            lambda x: x["graph"]["digest"].update(version="0.11.0"), lambda x: x["tests"].pop(),
            lambda x: x["commands"]["build"]["termination"].update(exitCode=1),
            lambda x: x["commands"]["probe"]["command"]["args"].append("--skip"),
            lambda x: x["commands"]["probe"]["termination"].update(error="ETIMEDOUT")]
        for mutation in mutations:
            value = self.fresh(); mutation(value.receipt)
            with self.assertRaises(AuditError): value.validate()

    def test_complete_test_transcript_has_no_missing_duplicate_skip_or_unbound_stream(self):
        for raw in [fixture.RAW.replace(b"3 passed", b"0 passed"), fixture.RAW.replace(b"0 ignored", b"1 ignored"),
                    fixture.RAW.replace(b" ... ok", b" ... FAILED", 1), fixture.RAW + b"extra\n",
                    fixture.RAW.replace(owner.TESTS[0].encode(), owner.TESTS[1].encode()), fixture.RAW + b"\x1b"]:
            with self.assertRaises(AuditError): owner.validate_test_output(raw, b"")
        owner.validate_test_output(fixture.RAW.replace(b"\n", b"\r\n"), b"")
        with self.assertRaises(AuditError): owner.validate_test_output(fixture.RAW, b"warning")
        value = self.fresh(); (value.output / "block-buffer-probe.stdout.log").write_bytes(fixture.RAW + b"extra\n")
        with self.assertRaises(AuditError): value.validate()
        value = self.fresh(); value.receipt["commands"]["probe"]["logs"]["stdout"]["sha256"] = "0" * 64
        with self.assertRaises(AuditError): value.validate()
        value = self.fresh(); (value.output / "binary").write_bytes(b"substituted")
        with self.assertRaises(AuditError): value.validate()

    def test_owning_source_mutation_and_failed_actual_call_cannot_mint(self):
        value = self.fresh()
        def altered(*_args):
            value.source.write_bytes(b"changed"); return value.receipt
        with patch.object(owner, "PINS", value.pins), patch.object(owner, "_call", side_effect=altered):
            with self.assertRaises(AuditError): owner.qualify_not_affected(value.tools["node"]["path"], value.root,
                value.audit_output, value.tools["cargo"]["path"], value.tools["rustc"]["path"])
        value = self.fresh(); value.receipt["status"] = "failed"
        with self.assertRaises(AuditError): self.complete(value)

    def test_collect_preserves_admitted_rust_tools_separately_from_registry_queries(self):
        value = self.fresh(); completed = self.complete(value)
        cargo, rustc, node = (value.tools[name]["path"] for name in ["cargo", "rustc", "node"])
        class FakeCapture:
            def __init__(self, _output): pass
            def request(self, path, _name, _body=None, **_kwargs):
                return encode({"results": [{"vulns": fixture.MATCH[0]["advisories"]}]}) \
                    if path == "querybatch" else encode(fixture.DETAIL)
        def material(path, _limit=None):
            return {"path": str(path), "bytes": b"synthetic-only", "sha256": sha(b"synthetic-only"),
                    "identity": (1, 2, 14, 1, 1, 1, 1)}
        parsed = {"format": "zryna.dependency-lock-inputs.v1", "node": "v22.22.1", "yamlVersion": "2.8.3",
                  "yamlEntrySha256": "0" * 64, "packages": []}
        with patch.multiple(gate, snapshot=material, unchanged=lambda *_args: None, Capture=FakeCapture,
                cargo_inputs=lambda *_args: ([owner.QUERY], [], []), node_call=lambda *_args: parsed), \
                patch.object(owner, "qualify_not_affected", return_value=completed) as actual_owner:
            report = gate.collect(node, value.root, value.audit_output, cargo, rustc)
        actual_owner.assert_called_once_with(node, value.root, value.audit_output, cargo, rustc)
        self.assertEqual(report["queries"], [owner.QUERY])
        self.assertEqual(report["toolVersions"]["cargoExecutable"], cargo)
        self.assertEqual(report["toolVersions"]["rustcExecutable"], rustc)
        self.assertEqual(report["observedDependencyGate"], "pass")
        self.assertEqual(len(report["rawAdvisoryMatches"]), 1)
        self.assertEqual(len(report["sourceBoundNotAffected"]), 1)


if __name__ == "__main__":
    unittest.main()
