"""F1/F2/F3 synthetic controls; no network/Node/SDK or owner execution."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import test_audit as fixtures
import dependency_audit as gate
import dependency_audit_osv as osv
from dependency_audit_inputs import AuditError, decode, encode, sha


class ReviewRegressions(unittest.TestCase):
    def test_review_f1_timestamp_binding_before_owner_with_exact_retained_pair_only(self):
        def run_case(batch_modified, detail, reject=False, identifier=fixtures.ADVISORY):
            class FakeCapture:
                def __init__(self, _output): pass
                def request(self, path, _name, _body=None, **_kwargs):
                    return encode({'results': [{'vulns': [{'id': identifier, 'modified': batch_modified}]}]}) \
                        if path == 'querybatch' else encode(detail)
            def material(path, _limit=None):
                return {'path': str(path), 'bytes': b'synthetic-only', 'sha256': sha(b'synthetic-only'),
                        'identity': (1, 2, 14, 1, 1, 1, 1)}
            query = fixtures.QUERY if identifier == fixtures.ADVISORY else {
                'package': {'ecosystem': 'npm', 'name': 'ordinary'}, 'version': '1.0.0'}
            parsed = {'format': 'zryna.dependency-lock-inputs.v1', 'node': 'v22.22.1',
                      'yamlVersion': '2.8.3', 'yamlEntrySha256': '0' * 64, 'packages': [query]}
            with tempfile.TemporaryDirectory() as tmp, patch.multiple(gate, snapshot=material,
                    cargo_inputs=lambda *_args: ([], [], []), unchanged=lambda *_args: None,
                    node_call=lambda *_args: parsed, Capture=FakeCapture), patch.object(gate, 'qualify_backport', return_value=None) as owner:
                if reject:
                    with self.assertRaises(AuditError): gate.collect(Path('/synthetic/node'), Path('/synthetic/root'), Path(tmp))
                    self.assertFalse(owner.called, 'mismatched pair must stop BEFORE SDK execution')
                else:
                    report = gate.collect(Path('/synthetic/node'), Path('/synthetic/root'), Path(tmp))
                    self.assertTrue(owner.called, 'valid pair reaches fake owner without granting completion')
                    self.assertEqual(report['observedDependencyGate'], 'fail')
                self.assertTrue((Path(tmp) / 'RAW_ADVISORY_INDEX.json').exists(), 'retain raw matches first')
        # Honest exact equality and precisely the documented old provider pair; no generic rounding.
        run_case(fixtures.DETAIL['modified'], fixtures.DETAIL)
        run_case('2026-10-23T22:30:05.010413Z', fixtures.DETAIL)
        run_case('2026-10-23T22:30:05.010414Z', fixtures.DETAIL, True)
        run_case('2026-10-23T22:30:05.010413Z', {**fixtures.DETAIL,
                 'modified': '2026-10-23T22:30:05.010413242Z'}, True)
        run_case('2026-10-23T22:30:05.010413Z', {**fixtures.DETAIL, 'summary': 'changed full body'}, True)
        run_case('2026-10-23T22:30:05.010413Z', {**fixtures.DETAIL,
                 'id': 'GHSA-unknown', 'modified': '2026-10-23T22:30:05.010414Z'}, True, 'GHSA-unknown')

    def test_review_f2_late_eof_rejects_both_deadlines_after_raw_retention(self):
        for started, late in [(0.0, 46.0), (-590.0, 11.0)]:
            now = [0.0]
            with tempfile.TemporaryDirectory() as tmp, patch.object(osv.time, 'monotonic', lambda: now[0]):
                output = Path(tmp); capture = osv.Capture(output); capture.started = started
                response = fixtures.FakeResponse(b'{}')
                def read(count):
                    value = fixtures.FakeResponse.read1(response, count)
                    if not value: now[0] = late
                    return value
                response.read1 = read; capture.opener.open = lambda *_a, **_k: response
                with self.assertRaises(AuditError): capture.request('querybatch', 'late-eof', b'{}')
                self.assertEqual((output / 'late-eof.response.json').read_bytes(), b'{}')
                self.assertIsNotNone(decode((output / 'late-eof.http.json').read_bytes())['error'])

    def test_review_f3_pagination_absent_or_empty_string_only(self):
        self.assertEqual(len(osv.parse_batch(encode({'results': [{'next_page_token': ''}]}), [fixtures.QUERY])), 1)
        for token in [0, False, None, [], {}]:
            with self.assertRaises(AuditError):
                osv.parse_batch(encode({'results': [{'next_page_token': token}]}), [fixtures.QUERY])


if __name__ == '__main__':
    unittest.main()
