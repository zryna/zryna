"""Externally pinned selected plan -> full360 USTAR, using retained-source/USTAR primitives."""
import time
from cli_common import digest, integer, require
from cli_image_plan import MAX_PLAN, decode_plan, path
from cli_image_sources import Sources
from cli_image_archive import emit
from cli_image_materialize import materialize


def selected(sources, plan_path, plan_bytes, plan_sha256):
    path(plan_path, absolute=True); integer(plan_bytes, MAX_PLAN, 1); digest(plan_sha256)
    # This is the only read/hash performed before the external selector matches.
    raw = sources.read_plan(plan_path, plan_bytes, plan_sha256)
    return raw, decode_plan(raw)


def prepare(sources, plan_path, plan_bytes, plan_sha256, cache, cache_fd, deadline):
    raw, document = selected(sources, plan_path, plan_bytes, plan_sha256)
    for item in document['runtimeFiles']:
        sources.verify(item)
    require(time.monotonic() < deadline, 'finite CLI image preparation')
    captured = materialize(document, sources, cache, cache_fd, deadline)
    for item in captured['files']:
        sources.verify(item)
    sources.revalidate_files()
    require(sources.read_plan(plan_path, plan_bytes, plan_sha256) == raw and time.monotonic() < deadline,
            'externally selected plan unchanged after all source hashing')
    return raw, document, captured


def emit_selected(sources, raw, document, captured, plan_path, plan_bytes, plan_sha256, output, deadline):
    require(time.monotonic() < deadline and sources.read_plan(plan_path, plan_bytes, plan_sha256) == raw,
            'exact selected plan before any USTAR output')
    sources.verify(document['archive'])
    receipt = emit(captured, sources, output)
    sources.verify(document['archive'])
    require(sources.read_plan(plan_path, plan_bytes, plan_sha256) == raw and time.monotonic() < deadline,
            'complete before/after source and externally selected plan identity')
    receipt.update({'format': 'zryna.playground-cli-image-emission.v1', 'version': 1,
                    'planBytes': plan_bytes, 'planSha256': plan_sha256,
                    'originalReleaseFiles': 349, 'originalReleaseModesRetained': True,
                    'sourceInventoryBeforeAfterEqual': True, 'cliOrProviderExecuted': False,
                    'scope': 'image transport only; no runtime or publisher authority'})
    return receipt
