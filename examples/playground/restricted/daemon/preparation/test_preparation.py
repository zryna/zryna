"""Real filesystem/archive failure gates; no Docker, source execution or approval."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
import xml.etree.ElementTree as ET
from compose import candidate, encode
from emit import run
from handles import Sources
import ui_plan
from plan import MAX_PLAN, decode, validate


class Preparation(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='zryna-preparation-')
        self.root = Path(self.temporary.name)
        self.source = self.root / 'retained' / 'node'
        self.source.parent.mkdir()
        self.source.write_bytes(b'fixed inert fixture bytes\n')
        self.item = {'path': 'runtime/node/bin/node', 'sourcePath': str(self.source),
                     'bytes': self.source.stat().st_size,
                     'sha256': hashlib.sha256(self.source.read_bytes()).hexdigest(), 'mode': 365}
        self.document = candidate('materials', 'compiler', [self.item])

    def tearDown(self):
        self.temporary.cleanup()

    def invoke(self, document=None, output=None):
        raw, pin = encode(document or self.document)
        plan = self.root / 'plan.json'
        plan.write_bytes(raw)
        return run(str(plan), pin['bytes'], pin['sha256'], output or io.BytesIO())

    def test_actual_archive_ordinary_bytes_and_metadata(self):
        output = io.BytesIO()
        result = self.invoke(output=output)
        self.assertEqual(result['archiveSha256'], hashlib.sha256(output.getvalue()).hexdigest())
        self.assertTrue(result['sourceInventoryBeforeAfterEqual'])
        with tarfile.open(fileobj=io.BytesIO(output.getvalue()), mode='r:') as archive:
            members = archive.getmembers()
            self.assertEqual([m.name for m in members], ['.', *self.document['directories'], self.item['path']])
            for member in members:
                self.assertEqual((member.uid, member.gid, member.mtime), (0, 0, 0))
                self.assertEqual(member.mode, 365)
                self.assertFalse(member.issym() or member.islnk())
            self.assertEqual(archive.extractfile(self.item['path']).read(), self.source.read_bytes())

    def test_oversized_raw_plan_rejected_before_read_or_output(self):
        plan = self.root / 'oversized.json'
        with plan.open('wb') as stream:
            stream.truncate(MAX_PLAN + 1)
        output = io.BytesIO()
        with self.assertRaises(ValueError):
            run(str(plan), MAX_PLAN + 1, '0' * 64, output)
        self.assertEqual(output.getvalue(), b'')

    def test_external_plan_pin_required_before_output(self):
        raw, pin = encode(self.document)
        plan = self.root / 'plan.json'
        plan.write_bytes(raw)
        output = io.BytesIO()
        with self.assertRaises(ValueError):
            run(str(plan), pin['bytes'], '0' * 64, output)
        self.assertEqual(output.getvalue(), b'')

    def test_all_source_hashes_checked_before_first_header(self):
        second = dict(self.item, path='licenses/last', sha256='0' * 64)
        output = io.BytesIO()
        with self.assertRaises(ValueError):
            self.invoke(candidate('materials', 'compiler', [self.item, second]), output)
        self.assertEqual(output.getvalue(), b'')

    def test_source_parent_symlink_rejected_before_output(self):
        alias = self.root / 'alias'
        alias.symlink_to(self.source.parent, target_is_directory=True)
        altered = dict(self.item, sourcePath=str(alias / 'node'))
        output = io.BytesIO()
        with self.assertRaises(OSError):
            self.invoke(candidate('materials', 'compiler', [altered]), output)
        self.assertEqual(output.getvalue(), b'')

    def test_moved_ancestor_rejected_with_original_fd_retained(self):
        sources = Sources()
        try:
            sources.verify(self.item)
            self.source.parent.rename(self.root / 'moved')
            self.source.parent.mkdir()
            self.source.write_bytes(b'fixed inert fixture bytes\n')
            with self.assertRaises(ValueError):
                sources.verify(self.item)
        finally:
            sources.close()

    def test_same_size_content_change_rejected_after_preflight(self):
        sources = Sources()
        try:
            sources.verify(self.item)
            self.source.write_bytes(b'x' * self.item['bytes'])
            with self.assertRaises(ValueError):
                sources.verify(self.item)
        finally:
            sources.close()

    def test_live_mutation_during_emission_is_failure(self):
        source = self.source
        class MutatingOutput(io.BytesIO):
            def write(self, block):
                source.write_bytes(b'x' * source.stat().st_size)
                return super().write(block)
        with self.assertRaises(ValueError):
            self.invoke(output=MutatingOutput())

    def test_closed_paths_metadata_counts_and_totals(self):
        variants = []
        for key, value in [('bytes', self.document['bytes'] + 1), ('directories', []),
                           ('config', {}), ('version', True), ('extra', 1)]:
            altered = copy.deepcopy(self.document)
            altered[key] = value
            variants.append(altered)
        for key, value in [('path', '../escape'), ('mode', 511), ('bytes', 512 * 1024**2),
                           ('sourcePath', '/tmp//alias'), ('extra', 1)]:
            altered = copy.deepcopy(self.document)
            altered['files'][0][key] = value
            variants.append(altered)
        duplicate = copy.deepcopy(self.document)
        duplicate['files'] *= 2
        variants.append(duplicate)
        for altered in variants:
            with self.subTest(altered=altered), self.assertRaises(ValueError):
                validate(altered)
        with self.assertRaises(ValueError):
            decode(b'{"role":"compiler","role":"evaluator"}')
        with self.assertRaises(ValueError):
            decode(b'{"value":NaN}')


class UiPlanClosure(unittest.TestCase):
    def test_corpus_has_only_two_literal_required_inner_helpers(self):
        files = [{'path': name, 'sourcePath': '/tmp/fixed-inert-source', 'bytes': 1, 'sha256': 'a' * 64,
                  'mode': 365 if name in ('supervisor', 'ui-supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2') else 292}
                 for name in ui_plan.REQUIRED]
        for name in ['app/browser-corpus.mjs', 'app/browser-oracles.mjs']:
            for replacement in [None, {'path': 'app/arbitrary-corpus.mjs'}, {'mode': 365}]:
                with self.subTest(name=name, replacement=replacement), self.assertRaises(ValueError):
                    ui_plan.candidate([dict(item, **replacement) if item['path'] == name else item
                                      for item in files if item['path'] != name or replacement is not None])

    def test_fixed_diagnostics_module_is_required_without_opening_other_script_paths(self):
        files = [{'path': name, 'sourcePath': '/tmp/fixed-inert-source', 'bytes': 1, 'sha256': 'a' * 64,
                  'mode': 365 if name in ('supervisor', 'ui-supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2') else 292}
                 for name in ui_plan.REQUIRED]
        self.assertIn('app/browser-diagnostics.mjs', [item['path'] for item in ui_plan.candidate(files)['files']])
        without = [item for item in files if item['path'] != 'app/browser-diagnostics.mjs']
        with self.assertRaises(ValueError):
            ui_plan.candidate(without)
        for replacement in [{'path': 'app/arbitrary-debug.mjs'}, {'mode': 365}]:
            changed = [dict(item, **replacement) if item['path'] == 'app/browser-diagnostics.mjs' else item for item in files]
            with self.assertRaises(ValueError):
                ui_plan.candidate(changed)


    def test_fixed_font_root_requires_each_font_config_license_and_rejects_other_paths(self):
        files = [{'path': name, 'sourcePath': '/tmp/fixed-inert-source', 'bytes': 1, 'sha256': 'a' * 64,
                  'mode': 365 if name in ('supervisor', 'ui-supervisor', 'app/compiler', 'lib64/ld-linux-x86-64.so.2') else 292}
                 for name in ui_plan.REQUIRED]
        for name in ['etc/fonts/fonts.conf', 'resources/fonts/NimbusSans-Regular.otf',
                     'resources/fonts/NimbusMonoPS-Regular.otf', 'licenses/fonts/fonts-urw-base35-copyright']:
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    ui_plan.candidate([item for item in files if item['path'] != name])
                for replacement in [{'path': name + '.other'}, {'mode': 365}]:
                    with self.assertRaises(ValueError):
                        ui_plan.candidate([dict(item, **replacement) if item['path'] == name else item for item in files])
        config = Path(__file__).with_name('ui-fonts.conf').read_bytes()
        self.assertNotIn(b'<!DOCTYPE', config);self.assertNotIn(b'<!ENTITY', config)
        root = ET.fromstring(config);self.assertEqual(root.tag, 'fontconfig')
        self.assertTrue(all(not element.attrib for element in root.iter()))
        self.assertEqual([element.text for element in root.findall('dir')], ['/resources/fonts'])
        self.assertEqual([element.text for element in root.findall('cachedir')], ['/tmp/zryna-fontconfig-cache'])
        self.assertEqual({element.tag for element in root.iter()}, {'fontconfig', 'dir', 'cachedir', 'alias', 'family', 'prefer'})
        aliases = {alias.findtext('family'): alias.findtext('prefer/family') for alias in root.findall('alias')}
        self.assertEqual(aliases, {**dict.fromkeys(['Sans', 'Arial', 'sans-serif', 'system-ui'], 'Nimbus Sans'),
                                  **dict.fromkeys(['monospace', 'ui-monospace'], 'Nimbus Mono PS')})


if __name__ == '__main__':
    unittest.main()
