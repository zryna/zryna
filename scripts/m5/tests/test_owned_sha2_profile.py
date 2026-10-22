"""Pure package/profile-selection controls; no compiler or runtime execution."""
from copy import deepcopy
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import owned_compile
import test_owned_compile as existing


class Sha2PackageProfile(unittest.TestCase):
    def fixture(self):
        fixture = list(existing.OwnedCompilerInputs().fixture())
        repo, deps, metadata, messages, manifest = fixture
        package = metadata['packages'][0]
        package['dependencies'].append({'name': 'sha2', 'rename': None, 'kind': None,
            'target': None, 'features': [], 'uses_default_features': True})
        metadata['packages'].append({'id': owned_compile.SHA2_PACKAGE, 'name': 'sha2',
            'version': '0.10.9', 'source': owned_compile.SHA2_SOURCE,
            'features': {'default': ['std'], 'std': ['digest/std']}})
        metadata['resolve']['nodes'][0]['deps'].append({'name': 'sha2',
            'pkg': owned_compile.SHA2_PACKAGE, 'dep_kinds': [{'kind': None, 'target': None}]})
        metadata['resolve']['nodes'].append({'id': owned_compile.SHA2_PACKAGE,
                                           'features': ['default', 'std']})
        messages.append({'reason': 'compiler-artifact', 'package_id': owned_compile.SHA2_PACKAGE,
            'target': {'kind': ['lib']}, 'profile': {'opt_level': '3', 'test': False, 'debuginfo': 0},
            'features': ['default', 'std'], 'filenames': [str(deps / 'libsha2-reviewed.rlib')]})
        fixture[4] = manifest.replace(b'serde.workspace=true',
                                     b'serde.workspace=true\nsha2.workspace=true')
        return fixture

    def select(self, fixture):
        return existing.OwnedCompilerInputs().select(fixture)

    def test_exact_registry_sha2_profile_and_features_are_recorded(self):
        rows, _ = self.select(self.fixture())
        sha2 = next(row for row in rows if row['name'] == 'sha2')
        self.assertEqual(sha2['packageID'], owned_compile.SHA2_PACKAGE)
        self.assertEqual(sha2['features'], ['default', 'std'])
        self.assertEqual(sha2['profile'], {'opt_level': '3', 'test': False, 'debuginfo': 0})
        self.assertTrue(all(row['profile']['opt_level'] == '0'
                            for row in rows if row['name'] != 'sha2'))

    def test_sha2_unoptimized_or_other_optimization_refuses(self):
        for value in ('0', '2', 's', 3):
            fixture = self.fixture()
            fixture[3][-1]['profile']['opt_level'] = value
            with self.subTest(value=value), self.assertRaises(AssertionError):
                self.select(fixture)

    def test_wrong_registry_package_identity_refuses(self):
        for key, value in [('id', 'other-id'), ('name', 'other-name'),
                           ('version', '0.10.8'), ('source', 'git+https://invalid/source')]:
            fixture = self.fixture()
            fixture[2]['packages'][-1][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError):
                self.select(fixture)

    def test_matching_wrong_resolve_and_artifact_features_cannot_gain_exception(self):
        for features in (['default'], ['std'], ['asm', 'default', 'std']):
            fixture = self.fixture()
            fixture[2]['resolve']['nodes'][-1]['features'] = features
            fixture[3][-1]['features'] = features
            with self.subTest(features=features), self.assertRaises(AssertionError):
                self.select(fixture)

    def test_other_direct_package_does_not_gain_optimized_profile(self):
        fixture = self.fixture()
        fixture[3][0]['profile']['opt_level'] = '3'
        with self.assertRaises(AssertionError):
            self.select(fixture)

    def test_sha2_test_debug_and_duplicate_full_artifacts_refuse(self):
        for change in ({'test': True}, {'debuginfo': 1}):
            fixture = self.fixture()
            fixture[3][-1]['profile'].update(change)
            with self.subTest(change=change), self.assertRaises(AssertionError):
                self.select(fixture)
        fixture = self.fixture()
        duplicate = deepcopy(fixture[3][-1])
        duplicate['filenames'] = [str(fixture[1] / 'libsha2-second.rlib')]
        fixture[3].append(duplicate)
        with self.assertRaises(AssertionError):
            self.select(fixture)

    def test_workspace_declared_profile_refuses_before_tool_reads_or_commands(self):
        template = '[profile.dev.package.sha2]\nopt-level={}\n[workspace.dependencies]\nsha2="=0.10.9"\n'
        invalid = [template.format(value).encode() for value in ('0', '2', '"3"', 'true', '3.0')]
        invalid += [b'[workspace.dependencies]\nsha2="=0.10.9"\n',
                    template.format('3').replace('=0.10.9', '=0.10.8').encode(),
                    template.format('3').replace('opt-level=3', 'opt-level=3\ndebug=0').encode()]
        for document in invalid:
            with self.subTest(document=document), patch.object(owned_compile, 'read', return_value=document), \
                    patch.object(owned_compile, 'rust_tools') as tools, patch.object(owned_compile, 'run') as run:
                with self.assertRaises(AssertionError):
                    owned_compile.compile_owned('/fixed/repo', '/fixed/output', object(), {})
                tools.assert_not_called()
                run.assert_not_called()
