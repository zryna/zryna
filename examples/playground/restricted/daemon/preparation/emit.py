"""CLI: externally pinned plan -> stdout USTAR; bounded summary -> stderr, no Docker calls."""
import argparse
import hashlib
import json
import sys
from archive import emit
from handles import Sources
from plan import MAX_PLAN, decode, digest, integer, path, validate


def run(plan_path, plan_bytes, plan_sha256, output):
    path(plan_path, absolute=True)
    integer(plan_bytes, MAX_PLAN, 1)
    digest(plan_sha256)
    sources = Sources()
    try:
        raw = sources.read_plan(plan_path, plan_bytes, plan_sha256)
        document = validate(decode(raw))
        receipt = emit(document, sources, output)
        # Reopen and rehash the externally selected plan after every source use.
        if sources.read_plan(plan_path, plan_bytes, plan_sha256) != raw:
            raise ValueError('plan changed')
        receipt.update({'format': 'zryna.playground-archive-emission.v1', 'version': 1,
                        'kind': document['kind'], 'role': document['role'],
                        'planBytes': len(raw), 'planSha256': hashlib.sha256(raw).hexdigest(),
                        'sourceInventoryBeforeAfterEqual': True,
                        'scope': 'archive transport only; no installation or operator approval'})
    finally:
        sources.close()
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('plan')
    parser.add_argument('--plan-bytes', required=True, type=int)
    parser.add_argument('--plan-sha256', required=True)
    args = parser.parse_args()
    receipt = run(args.plan, args.plan_bytes, args.plan_sha256, sys.stdout.buffer)
    print(json.dumps(receipt, separators=(',', ':')), file=sys.stderr)


if __name__ == '__main__':
    main()
