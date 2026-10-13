"""Independent source-fixed tool selection; serialized pins grant no execution authority."""
import os
from capture_providers import walk, copy_hash, tool_identity, TOOL_ROOTS

ALIASES = ('/usr/bin/gcc', '/usr/bin/ld', '/usr/bin/objdump')


def select(pins):
    assert len(pins) == len(ALIASES)
    selected = []
    for alias, expected in zip(ALIASES, pins):
        fd, canonical, epoch = walk(alias, TOOL_ROOTS)
        try:
            identity = tool_identity(os.fstat(fd))
            length, digest, prefix = copy_hash(fd, 64 * 1024 * 1024)
            assert prefix.startswith(b'\x7fELF')
            assert canonical == expected['path'] and identity == expected['identity']
            assert (length, digest) == (expected['bytes'], expected['sha256'])
            selected.append({'sourceAlias': alias, 'canonicalPath': canonical,
                             'identity': identity, 'bytes': length,
                             'sha256': digest, 'componentEpoch': epoch})
        finally:
            os.close(fd)
    assert len({x['canonicalPath'] for x in selected}) == len(ALIASES)
    return selected
