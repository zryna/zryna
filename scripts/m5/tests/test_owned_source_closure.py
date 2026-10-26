"""Inert inventories test include closure; none assert Git or compiler provenance."""
from pathlib import Path
import hashlib
import json
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from owned_source_closure import audit, references
from owned_inputs import mirror, check_mirror


def source_inventory(repo, inventory=None):
    """Local test inventory only; generated files are not authenticated source."""
    if inventory is not None:
        return inventory()  # Explicit inert inventories used by isolated tests only.
    if (repo / '.git').exists():
        from collect_ordinary_sanitizer_prefix import REPO, git, source
        assert repo == REPO
        return source(git('rev-parse', 'HEAD').decode().strip())['files']
    # A plain source export has no Git authority. Enumerate it completely for
    # local coverage; hosted checkouts always use the production census above.
    rows = []
    for path in sorted(repo.rglob('*')):
        if path.is_file() and '.git' not in path.relative_to(repo).parts:
            data = path.read_bytes()
            rows.append({'path': str(path.relative_to(repo)), 'bytes': len(data),
                         'sha256': hashlib.sha256(data).hexdigest()})
    return rows


class SourceClosure(unittest.TestCase):
    def test_literals_comments_raw_strings_and_all_platform_branches(self):
        source = '''
        // include_str!("decoy");
        /* nested /* include!("decoy"); */ comment */
        const DECOY: &str = r###"include_bytes!("decoy")"###;
        #[cfg(windows)] const WINDOWS: &str = include_str!("windows.txt");
        #[cfg(unix)] const UNIX: &[u8] = include_bytes!(r#"unix.txt"#);
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/", "module.rs"));
        #[path = "nested/child.rs"] mod child;
        #[cfg_attr(windows, path = "windows/child.rs")] mod platform;
        #[doc = include_str!("documentation.txt")] struct Documented;
        '''
        self.assertEqual(references(source), ['windows.txt', 'unix.txt',
                         '@MANIFEST@/../../tests/module.rs', 'nested/child.rs',
                         'windows/child.rs', 'documentation.txt'])

    def test_source_defined_literal_macro_expands_each_include(self):
        source = '''
        macro_rules! fixture {
            ($directory:literal, $name:literal) => {
                include_bytes!(concat!("fixtures/", $directory, "/", $name, ".json"))
            };
        }
        const A: &[u8] = fixture!("windows", "one");
        const B: &[u8] = fixture!("unix", "two");
        '''
        self.assertEqual(references(source), ['fixtures/windows/one.json', 'fixtures/unix/two.json'])

    def test_unknown_expression_or_include_macro_form_fails_closed(self):
        for source in ('include_str!(unknown!("fixture"));',
                       'include_str!(env!("UNREVIEWED_ROOT"));',
                       'include_str!(concat!("fixture", $unbound));',
                       'macro_rules! x { ($p:expr) => { include_str!($p) }; } x!("a");',
                       'macro_rules! x { () => { include_str!("a") }; () => { 0 }; }'):
            with self.subTest(source=source), self.assertRaises(AssertionError):
                references(source)

    def test_nested_external_rust_path_and_inactive_include_require_full_closure(self):
        files = {
            'crates/zryna-driver/src/lib.rs': '#[path = "../../../tests/hostile.rs"] mod hostile;',
            'tests/hostile.rs': '#[path = "nested.rs"] mod nested;',
            'tests/nested.rs': '#[cfg(windows)] const X: &str = include_str!("windows.txt");',
            'tests/windows.txt': 'full source fixture',
        }
        rows = [{'path': path} for path in files]
        load = lambda path: files[path].encode()
        complete = audit(rows, load, set(files))
        self.assertEqual(len(complete['includePathEdges']), 3)
        self.assertEqual(complete['rustFilesExamined'], sorted(path for path in files if path.endswith('.rs')))
        with self.assertRaisesRegex(AssertionError, 'missing from owned mirror closure'):
            audit(rows, load, set(files) - {'tests/windows.txt'})
        with self.assertRaisesRegex(AssertionError, 'missing from complete source census'):
            audit(rows[:-1], load, set(files) - {'tests/windows.txt'})

    def test_actual_full_tree_mirror_retains_all_selected_bytes_and_audits_every_omission(self):
        # The checkout may be an honest plain-source tar export. This inventory is
        # only a local test input, never a substitute for hosted source() Git checks.
        repo = Path(__file__).resolve().parents[3]
        rows = source_inventory(repo)
        original_rows = [dict(row) for row in rows]
        with tempfile.TemporaryDirectory() as directory:
            root, manifest = mirror(repo, Path(directory), {'files': rows})
            check_mirror(repo, root, manifest)
            report = json.loads((Path(directory) / 'SOURCE_MIRROR.json').read_bytes())
            selected = {row['path'] for row in manifest}
            self.assertLessEqual(sum(row['originalBytes'] for row in manifest), 16 * 1024 * 1024)
            self.assertEqual(report['omittedSourcePaths'], sorted({row['path'] for row in rows} - selected))
            self.assertEqual(len(selected) + len(report['omittedSourcePaths']), len(rows))
            self.assertIn('tests/m7-generic-owned-native/hostile.rs', selected)
            self.assertIn('tests/native-c-rust-v0/library.rs', selected)
            required_edges = {
                ('crates/zryna-driver/src/lib.rs',
                 'crates/zryna-backend-native/tests/recursive_arena/source.rs'),
                ('crates/zryna-backend-native/tests/recursive_arena/source.rs',
                 'crates/zryna-backend-native/tests/recursive_arena/input-pins.json'),
                ('crates/zryna-driver/src/generic_run/tests/source.rs',
                 'tests/m7-generic-full-profile-first-vertical/literals/scalar-i32-clone.zry'),
            }
            self.assertTrue({target for _, target in required_edges} <= selected)
            self.assertTrue(required_edges <= {
                (edge['from'], edge['to']) for edge in report['closureAudit']['includePathEdges']})
            self.assertIn('tests/m7-generic-owned-nominal-enum/1-reference.json', report['omittedSourcePaths'])
            self.assertIn('tests/m7-generic-owned-native/hostile.rs', report['closureAudit']['rustFilesExamined'])
            self.assertTrue(report['closureAudit']['includePathEdges'])
            for edge in report['closureAudit']['includePathEdges']:
                self.assertIn(edge['to'], selected)
            self.assertEqual(rows, original_rows, 'source census cannot be filtered or mutated')

    def test_dirty_workspace_uses_complete_supplied_source_census_without_cache_paths(self):
        # These are inert test rows, not fake Git records. One supplied source path
        # intentionally lives under node_modules to reject directory blacklists.
        from owned_inputs import SOURCE_FILES
        with tempfile.TemporaryDirectory() as directory:
            repo, output = Path(directory) / 'source', Path(directory) / 'output'
            repo.mkdir()
            output.mkdir()
            for relative in (*SOURCE_FILES, 'node_modules/tracked-fixture.json'):
                path = repo / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'// inert source fixture')
            tracked = source_inventory(repo)
            cache = repo / 'node_modules/.pnpm' / ('p' * 180) / ('q' * 180) / ('r' * 180)
            cache.mkdir(parents=True)
            for index in range(2000):
                (cache / f'{index:05}.cache').write_bytes(b'')
            self.assertGreater(len(json.dumps([str(path.relative_to(repo))
                                              for path in repo.rglob('*.cache')]).encode()),
                               1024 * 1024)
            rows = source_inventory(repo, inventory=lambda: tracked)
            root, manifest = mirror(repo, output, {'files': rows})
            check_mirror(repo, root, manifest)
            report = json.loads((output / 'SOURCE_MIRROR.json').read_bytes())
            self.assertEqual(rows, tracked)
            self.assertEqual(report['omittedSourcePaths'], ['node_modules/tracked-fixture.json'])
            self.assertEqual({row['path'] for row in manifest},
                             {row['path'] for row in tracked} - {'node_modules/tracked-fixture.json'})

    def test_current_m7_compile_dependencies_reject_each_omission_without_broad_roots(self):
        from owned_inputs import SOURCE_FILES, SOURCE_ROOTS
        required = {
            'crates/zryna-backend-native/tests/recursive_arena/source.rs',
            'crates/zryna-backend-native/tests/recursive_arena/input-pins.json',
            'tests/m7-generic-full-profile-first-vertical/literals/scalar-i32-clone.zry',
        }
        self.assertTrue(required <= set(SOURCE_FILES))
        self.assertEqual(SOURCE_ROOTS,
                         ('crates/zryna-driver/src', 'runtime', 'spec', 'adapters', 'examples'))
        repo = Path(__file__).resolve().parents[3]
        origin = 'crates/zryna-driver/src/lib.rs'
        reference = '../../zryna-backend-native/tests/recursive_arena/source.rs'
        self.assertIn(reference, references((repo / origin).read_text()))
        # An explicitly inert lexical model retains the actual included-file bytes.
        # It does not represent Git, a Rust compilation, or a positive execution issuer.
        files = {origin: '#[cfg(any())] #[path = ' + json.dumps(reference) + '] mod fixture;'}
        for path in (*sorted(required), 'crates/zryna-driver/src/generic_run/tests/source.rs'):
            files[path] = (repo / path).read_text()
        rows = [{'path': path} for path in files]
        load = lambda path: files[path].encode()
        complete = audit(rows, load, set(files))
        self.assertEqual(len(complete['includePathEdges']), 3)
        for omitted in sorted(required):
            with self.subTest(omitted=omitted):
                with self.assertRaisesRegex(AssertionError, 'missing from owned mirror closure'):
                    audit(rows, load, set(files) - {omitted})
                with self.assertRaisesRegex(AssertionError, 'missing from complete source census'):
                    audit([row for row in rows if row['path'] != omitted], load, set(files))
