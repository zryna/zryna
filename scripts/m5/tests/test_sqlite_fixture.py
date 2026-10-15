"""Offline adversarial acquisition tests; no network or native execution."""
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import prepare_sqlite_fixture as fixture


class SQLiteIntakeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.spec = copy.deepcopy(fixture.SPEC)
        macros = ('#define SQLITE_VERSION "3.54.0"\n'
                  '#define SQLITE_VERSION_NUMBER 3054000\n'
                  '#define SQLITE_SOURCE_ID "' + self.spec['sourceId'] + '"\n').encode()
        self.source = {'sqlite3.c': macros + b'/* source */\n',
                       'sqlite3.h': macros + b'/* header */\n'}
        for name, value in self.source.items():
            self.spec['files'][name]['bytes'] = len(value)
            self.spec['files'][name]['sha256'] = hashlib.sha256(value).hexdigest()
        self.data = self.zip_bytes()
        self.pin_archive(self.data)
        self.override = patch.object(fixture, 'SPEC', self.spec)
        self.override.start()
        self.addCleanup(self.override.stop)
        self.original_download = fixture.download
        self.network = patch.object(fixture, 'download', side_effect=AssertionError('unexpected network'))
        self.network.start()
        self.addCleanup(self.network.stop)

    def zip_bytes(self, extra=(), omit=(), mutations=None):
        output = io.BytesIO()
        root = self.spec['archiveRoot'] + '/'
        members = [(root, b'', stat.S_IFDIR | 0o755)]
        members += [(root + name, self.source.get(name, b'other source'), stat.S_IFREG | 0o644)
                    for name in self.spec['members'] if name not in omit]
        members += list(extra)
        with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            for name, value, mode in members:
                info = zipfile.ZipInfo(name)
                info.create_system = 3
                info.external_attr = mode << 16
                if mutations:
                    mutations(info)
                archive.writestr(info, value)
        return output.getvalue()

    def pin_archive(self, data):
        self.spec['archiveBytes'] = len(data)
        self.spec['archiveSha3_256'] = hashlib.sha3_256(data).hexdigest()

    def local_archive(self, data=None):
        path = self.root / 'fixture.zip'
        path.write_bytes(self.data if data is None else data)
        return path

    def test_checked_in_manifest_pins_official_bytes_and_version(self):
        real = json.loads(fixture.SPEC_PATH.read_bytes())
        self.assertEqual(real['url'], 'https://www.sqlite.org/2026/sqlite-amalgamation-3540000.zip')
        self.assertEqual(real['archiveBytes'], 2990479)
        self.assertEqual(real['archiveSha3_256'],
                         '7b670a62fdfbd672b75fef004cb703c8a3e87d3a5cc7d675b4a08337004a2d93')
        self.assertEqual(real['files']['sqlite3.c']['sha256'],
                         '2da161e310ef0aba3465f22a4d12b8a43723236f0942fa76e8aac523c3317243')
        self.assertEqual(real['files']['sqlite3.h']['sha256'],
                         'cd20a2877bc82b94e36c1093b727dd87cfa3c1d581aac6e240925535cbd90166')
        self.assertEqual((real['version'], real['versionNumber']), ('3.54.0', 3054000))
        self.assertEqual(real['sourceId'],
                         '2026-10-09 15:46:58 be8d059e9a49089ab2dce5ed26dd87aaf598fdc5fbe0b107c0dd758464bcd5e3')

    def test_archive_checksum_is_checked_before_zip_parser(self):
        bad = self.data[:-1] + bytes([self.data[-1] ^ 1])
        with patch.object(fixture, 'unpack') as unpack, self.assertRaisesRegex(ValueError, 'SHA3'):
            fixture.authenticate_archive(bad)
        unpack.assert_not_called()

    def test_wrong_archive_size_and_download_ceiling_reject(self):
        with self.assertRaisesRegex(ValueError, 'byte count'):
            fixture.authenticate_archive(self.data + b'x')
        self.spec['limits']['archiveBytes'] = len(self.data) - 1
        with self.assertRaisesRegex(ValueError, 'ceiling'):
            fixture.authenticate_archive(self.data)

    def test_valid_offline_intake_and_cache_reuse_never_overwrite(self):
        out = self.root / 'cache'
        fixture.prepare(out, self.local_archive())
        before = {p.name: (p.stat().st_ino, p.stat().st_mtime_ns, p.read_bytes()) for p in out.iterdir()}
        self.assertEqual(fixture.prepare(out), out)
        self.assertEqual(before, {p.name: (p.stat().st_ino, p.stat().st_mtime_ns, p.read_bytes())
                                  for p in out.iterdir()})
        self.assertEqual(stat.S_IMODE(out.stat().st_mode), 0o700)
        for name, value in self.source.items():
            self.assertEqual((out / name).read_bytes(), value)
            self.assertEqual(stat.S_IMODE((out / name).stat().st_mode), 0o400)

    def test_each_changed_cache_file_rejects_without_repair(self):
        archive = self.local_archive()
        for index, name in enumerate(['archive.zip', 'sqlite3.c', 'sqlite3.h', 'receipt.json']):
            out = self.root / ('cache-' + str(index))
            fixture.prepare(out, archive)
            path = out / name
            original = path.read_bytes()
            path.chmod(0o600)
            path.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
            path.chmod(0o400)
            changed = path.read_bytes()
            with self.assertRaises(ValueError):
                fixture.prepare(out)
            self.assertEqual(path.read_bytes(), changed)

    def test_incomplete_and_extra_cache_entries_reject(self):
        out = self.root / 'incomplete'
        out.mkdir(mode=0o700)
        with self.assertRaisesRegex(ValueError, 'inventory'):
            fixture.prepare(out)
        good = self.root / 'good'
        fixture.prepare(good, self.local_archive())
        (good / 'unexpected').write_bytes(b'not a source')
        with self.assertRaisesRegex(ValueError, 'inventory'):
            fixture.prepare(good)

    def test_changed_permissions_symlinks_and_hardlinks_reject(self):
        archive = self.local_archive()
        for index, mode in enumerate(['permission', 'symlink', 'hardlink']):
            out = self.root / ('cache-' + str(index))
            fixture.prepare(out, archive)
            path = out / 'sqlite3.c'
            if mode == 'permission':
                path.chmod(0o600)
            else:
                target = self.root / ('target-' + str(index))
                path.rename(target)
                if mode == 'symlink':
                    path.symlink_to(target)
                else:
                    os.link(target, path)
            with self.assertRaises((ValueError, OSError)):
                fixture.prepare(out)

    def test_output_inside_checkout_rejects_before_creating_anything(self):
        output = Path(fixture.__file__).resolve().parents[2] / 'forbidden-sqlite-output'
        with self.assertRaisesRegex(ValueError, 'outside checkout'):
            fixture.prepare(output)
        self.assertFalse(output.exists())

    def test_output_symlink_ancestor_and_unsafe_permissions_reject(self):
        real = self.root / 'real'
        real.mkdir(mode=0o700)
        link = self.root / 'link'
        link.symlink_to(real, target_is_directory=True)
        for output in [link, link / 'child']:
            with self.assertRaises(OSError):
                fixture.prepare(output)
        real.chmod(0o755)
        with self.assertRaisesRegex(ValueError, 'mode0700'):
            fixture.prepare(real)
        self.assertFalse((real / 'child').exists())

    def test_duplicate_and_traversal_zip_names_reject(self):
        root = self.spec['archiveRoot'] + '/'
        for name in [root + 'sqlite3.c', '../sqlite3.c', '/sqlite3.c', root + '..\\sqlite3.c']:
            with self.subTest(name=name):
                data = self.zip_bytes(extra=[(name, b'x', stat.S_IFREG | 0o644)])
                self.pin_archive(data)
                with self.assertRaises(ValueError):
                    fixture.authenticate_archive(data)

    def test_missing_required_member_rejects(self):
        data = self.zip_bytes(omit=['sqlite3.h'])
        self.pin_archive(data)
        with self.assertRaisesRegex(ValueError, 'member'):
            fixture.authenticate_archive(data)

    def test_linked_and_special_zip_member_reject(self):
        for mode in [stat.S_IFLNK | 0o777, stat.S_IFIFO | 0o600]:
            data = self.zip_bytes(mutations=lambda info: setattr(info, 'external_attr', mode << 16)
                                  if info.filename.endswith('sqlite3.c') else None)
            self.pin_archive(data)
            with self.assertRaisesRegex(ValueError, 'special'):
                fixture.authenticate_archive(data)

    def test_entry_and_expanded_size_limits_reject(self):
        for key, value in [('entries', 4), ('expandedBytes', 1)]:
            with patch.dict(self.spec['limits'], {key: value}), self.assertRaises(ValueError):
                fixture.authenticate_archive(self.data)

    def test_source_size_sha_and_version_checks_are_independent(self):
        fixed = self.spec['files']['sqlite3.c']
        for key, value in [('bytes', fixed['bytes'] + 1), ('maxBytes', fixed['bytes'] - 1),
                           ('sha256', '0' * 64)]:
            with patch.dict(fixed, {key: value}), self.assertRaises(ValueError):
                fixture.authenticate_archive(self.data)
        for key, value in [('version', '3.53.0'), ('versionNumber', 3540000), ('sourceId', 'invented')]:
            with patch.dict(self.spec, {key: value}), self.assertRaisesRegex(ValueError, 'source-id'):
                fixture.authenticate_archive(self.data)

    def test_failed_intake_retains_empty_private_directory_without_source_writes(self):
        out = self.root / 'rejected'
        path = self.local_archive(self.data[:-1] + b'x')
        with self.assertRaises(ValueError):
            fixture.prepare(out, path)
        self.assertEqual(list(out.iterdir()), [])
        self.assertEqual(stat.S_IMODE(out.stat().st_mode), 0o700)

    def test_http_valid_body_preserves_fixed_url_and_finite_timeout(self):
        test = self
        class Response:
            status = 200
            headers = {'Content-Length': str(len(test.data))}
            def __enter__(self):
                self.stream = io.BytesIO(test.data)
                return self
            def __exit__(self, *args): return False
            def geturl(self): return fixture.SPEC['url']
            def read(self, count): return self.stream.read(count)
        class Opener:
            def open(self, request, timeout):
                test.assertEqual(request.full_url, test.spec['url'])
                test.assertEqual(timeout, 20)
                test.assertEqual(request.get_header('Accept-encoding'), 'identity')
                return Response()
        with patch.object(fixture.urllib.request, 'build_opener', return_value=Opener()):
            self.assertEqual(self.original_download(), self.data)

    def test_http_total_deadline_is_enforced(self):
        class Response:
            status = 200
            headers = {}
            def __enter__(self): return self
            def __exit__(self, *args): return False
            def geturl(self): return fixture.SPEC['url']
        class Opener:
            def open(self, request, timeout): return Response()
        with patch.object(fixture.urllib.request, 'build_opener', return_value=Opener()), \
                patch.object(fixture.time, 'monotonic', side_effect=[0, 61]), \
                self.assertRaisesRegex(ValueError, 'deadline'):
            self.original_download()

    def test_download_rejects_redirect(self):
        with self.assertRaisesRegex(ValueError, 'redirect'):
            fixture.NoRedirect().redirect_request(None, None, 302, '', {}, 'http://elsewhere/')

    def test_http_shape_and_oversized_body_fail_offline(self):
        class Response:
            status = 200
            headers = {}
            def __enter__(self): return self
            def __exit__(self, *args): return False
            def geturl(self): return fixture.SPEC['url']
            def read(self, count): return b'x' * count
        class Opener:
            def open(self, request, timeout): return response
        # Call the original function, while its opener is an offline fake.
        original = self.original_download
        cases = [('status', 206), ('headers', {'Content-Encoding': 'gzip'}),
                 ('headers', {'Content-Length': '1'}), ('status', 200)]
        for field, value in cases:
            response = Response()
            setattr(response, field, value)
            with patch.object(fixture.urllib.request, 'build_opener', return_value=Opener()), \
                    self.assertRaises(ValueError):
                original()


if __name__ == '__main__':
    unittest.main()
