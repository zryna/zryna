"""Copy fixed ordinary-original failure families; never execute or retry a candidate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

CASES = {
    'nested-struct-v4': ('nested-copy', 'nested-owned-call', 'option-box-root', 'box-option-root',
                         'plain-and-reversed-arguments', 'phantom-remains-copy',
                         'fixed-member-remains-affine', 'nested-prefix-faults'),
    'struct-clone-v5': ('nested-prefix', 'repeat-source', 'shared-owner', 'phantom-opaque',
                        'fixed-member', 'family-fields', 'copy-struct'),
}
FIXED = {'program.mjs', 'program.wasm', 'program.o', 'program.zir', 'program-native', 'main.zry',
         'ownership-runtime-v1.h', 'provider-native.json', 'provider-typescript.json',
         'execution-oracle.json', 'fixture-identities.json', 'object-inventory-audit.json',
         'same-seal.json', 'original-expected.json', 'SOURCE_EXPECTED_FREEZE.json',
         'wire-freeze.json', 'native-observations.json'}
PORTABLE = re.compile(r'(javascript|webassembly)-(observations\.json|progress\.jsonl|'
                      r'process-request\.json|child\.stdout|child\.stderr|child-status\.json)\Z')
NATIVE = re.compile(r'native-(initial|fault-[1-9][0-9]?|retry-after-[1-9][0-9]?)'
                    r'\.(stdout|identities\.jsonl)\Z')
SNAPSHOT = re.compile(r'm416-process-deadline-[1-9][0-9]{0,9}-[1-9][0-9]{0,9}\.json\Z')
CASE = re.compile(r'm416-(nested-struct-v4|struct-clone-v5)-driver-([1-9][0-9]{0,9})-'
                  r'([a-z][a-z-]{0,63})-([0-9]{1,39})\Z')
SCAN, ROOTS, DIRECTORIES, FILES = 16384, 32, 32, 128
FILE_BYTES, TOTAL_BYTES = 4 * 1024 * 1024, 64 * 1024 * 1024


def identity(value):
    return (value.st_dev, value.st_ino, value.st_mode, value.st_size,
            value.st_mtime_ns, value.st_ctime_ns)


def issue(report, path, reason):
    if len(report['issues']) < 128:
        report['issues'].append({'path': str(path), 'reason': reason})
    else:
        report['issues_first_extra'] += 1

def copy_file(report, output, source, relative, parent_fd):
    row = {'source': str(source), 'retained': str(relative), 'bytes': 0, 'sha256': None}
    report['files'].append(row)
    try:
        before = os.stat(source.name, dir_fd=parent_fd, follow_symlinks=False)
        row.update(sourceMode=stat.S_IMODE(before.st_mode), sourceMtimeNs=before.st_mtime_ns)
        if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
            raise ValueError('not an exclusive regular original')
        if before.st_size > FILE_BYTES or report['bytes'] + before.st_size > TOTAL_BYTES:
            raise ValueError('copy byte cap; original remains unmodified')
        descriptor = os.open(source.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent_fd)
        with os.fdopen(descriptor, 'rb') as original:
            opened = os.fstat(original.fileno())
            if identity(opened) != identity(before):
                raise ValueError('original replaced before read')
            data = original.read(before.st_size + 1)
            if len(data) != before.st_size or identity(os.fstat(original.fileno())) != identity(opened):
                raise ValueError('original changed during read')
        if identity(os.stat(source.name, dir_fd=parent_fd, follow_symlinks=False)) != identity(before):
            raise ValueError('original changed after read')
        destination = output / relative
        destination.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        descriptor = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, 'wb') as retained:
            retained.write(data)
        row.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
        report['bytes'] += len(data)
    except (OSError, ValueError) as error:
        issue(report, source, str(error))

def copy_case(report, output, source, relative, parent_fd):
    if len(report['directories']) >= DIRECTORIES:
        issue(report, source, 'case directory first extra; selection incomplete')
        return
    report['directories'].append({'source': str(source), 'retained': str(relative)})
    (output / relative).mkdir(mode=0o700, parents=True, exist_ok=False)
    directory_fd = None
    try:
        before = os.stat(source.name, dir_fd=parent_fd, follow_symlinks=False)
        if not stat.S_ISDIR(before.st_mode):
            raise ValueError('case is not a regular directory')
        directory_fd = os.open(source.name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd)
        if identity(os.fstat(directory_fd)) != identity(before):
            raise ValueError('case replaced before scan')
        with os.scandir(directory_fd) as entries:
            for ordinal, entry in enumerate(entries):
                path = source / entry.name
                if ordinal == FILES:
                    issue(report, source, 'file first extra; selection incomplete')
                    break
                if not (entry.name in FIXED or PORTABLE.fullmatch(entry.name) or NATIVE.fullmatch(entry.name)):
                    issue(report, path, 'unknown case entry; no traversal or silent omission')
                    continue
                copy_file(report, output, path, relative / entry.name, directory_fd)
        after = os.stat(source.name, dir_fd=parent_fd, follow_symlinks=False)
        if identity(before) != identity(after):
            issue(report, source, 'case directory changed during collection')
    except (OSError, ValueError) as error:
        issue(report, source, str(error))
    finally:
        if directory_fd is not None:
            os.close(directory_fd)


def collect(output):
    output.mkdir(mode=0o700)
    report = {'format': 'zryna.original-generic-failure-copy.v1', 'scope': 'fixed-originals-only',
              'limits': {'globalScanEntries': SCAN, 'snapshotFiles': 32, 'issueRecords': 128, 'm4Roots': ROOTS, 'caseDirectories': DIRECTORIES,
                         'filesPerCase': FILES, 'fileBytes': FILE_BYTES, 'totalBytes': TOTAL_BYTES},
              'directories': [], 'files': [], 'issues': [], 'issues_first_extra': 0,
              'bytes': 0, 'globalM7': False}

    (output / 'collection-started.json').write_text(json.dumps({
        'status': 'incomplete-until-final-manifest', 'limits': report['limits'],
        'roots': ['/tmp', '/tmp/z4-[A-Za-z0-9]{6} one level only']}) + '\n')
    roots = [Path('/tmp')]
    m4_roots = 0
    snapshots = 0
    scanned = 0
    for root in roots:
        if scanned > SCAN:
            issue(report, root, 'global scan cap; root not collected')
            continue
        root_fd = None
        try:
            before = root.lstat()
            if not stat.S_ISDIR(before.st_mode):
                raise ValueError('scan root is not a regular directory')
            root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            if identity(os.fstat(root_fd))[:3] != identity(before)[:3]:
                raise ValueError('scan root replaced before open')
            with os.scandir(root_fd) as entries:
                for entry in entries:
                    scanned += 1
                    if scanned > SCAN:
                        issue(report, root, 'global scan first extra; selection incomplete')
                        break
                    source = root / entry.name
                    relative = Path(root.name) / entry.name
                    match = CASE.fullmatch(entry.name)
                    if match and match[3] in CASES[match[1]]:
                        copy_case(report, output, source, relative, root_fd)
                    elif SNAPSHOT.fullmatch(entry.name):
                        if snapshots == 32:
                            issue(report, source, 'snapshot first extra; selection incomplete')
                        else:
                            snapshots += 1
                            copy_file(report, output, source, relative, root_fd)
                    elif root == Path('/tmp') and entry.name.startswith('z4-'):
                        if not re.fullmatch(r'z4-[A-Za-z0-9]{6}', entry.name):
                            issue(report, source, 'unknown M4 temporary-root name')
                        elif m4_roots == ROOTS:
                            issue(report, source, 'M4 root first extra; selection incomplete')
                        elif entry.is_dir(follow_symlinks=False):
                            roots.append(source)
                            m4_roots += 1
                        else:
                            issue(report, source, 'M4 temporary root is not a regular directory')
                    elif entry.name.startswith(('m416-nested-struct-v4-driver-',
                                                'm416-struct-clone-v5-driver-',
                                                'm416-process-deadline-')):
                        issue(report, source, 'unknown original-family name; not traversed')
        except (OSError, ValueError) as error:
            issue(report, root, str(error))
        finally:
            if root_fd is not None:
                os.close(root_fd)
    report['status'] = 'incomplete' if report['issues'] or report['issues_first_extra'] else 'complete-for-fixed-selection'
    (output / 'manifest.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    print(f"Original failure copy: {report['status']}; {len(report['directories'])} directories; "
          f"{len(report['files'])} file records; {report['bytes']} retained bytes")
    return 1 if report['issues'] or report['issues_first_extra'] else 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    arguments = parser.parse_args()
    if not arguments.output.is_absolute():
        parser.error('absolute new RUNNER_TEMP output directory required')
    sys.exit(collect(arguments.output))
