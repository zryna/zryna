"""Bounded exact-file/registry inputs; no source, host or proof authority."""
import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat
import tomllib

MAX_JSON = 16 * 1024 * 1024
VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?")


class AuditError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise AuditError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, allow_nan=False,
                       separators=(",", ":")) + "\n").encode()


def decode(raw, maximum=MAX_JSON):
    require(isinstance(raw, bytes) and len(raw) <= maximum, "bounded JSON bytes")
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON field")
            result[key] = value
        return result
    def bad_constant(_):
        raise AuditError("nonfinite JSON")
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=unique, parse_constant=bad_constant)
    except (UnicodeError, RecursionError, json.JSONDecodeError) as error:
        raise AuditError("invalid JSON") from error
    pending, count = [(value, 0)], 0
    while pending:
        item, depth = pending.pop()
        count += 1
        require(count <= 100000 and depth <= 32, "bounded JSON shape")
        if isinstance(item, dict):
            pending.extend((v, depth + 1) for v in item.values())
        elif isinstance(item, list):
            pending.extend((v, depth + 1) for v in item)
        elif isinstance(item, float):
            require(math.isfinite(item), "nonfinite JSON number")
    return value


def identity(value):
    return (value.st_dev, value.st_ino, value.st_size, value.st_mode,
            value.st_nlink, value.st_mtime_ns, value.st_ctime_ns)


def snapshot(path, limit=4 * 1024 * 1024):
    path = Path(path)
    # Trusted ancestors, as with the existing evidence readers; no sandbox claim.
    for parent in (path, *path.parents):
        require(not parent.is_symlink(), "no persistent input symlinks")
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode) and before.st_size <= limit, "bounded regular input")
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0))
    with os.fdopen(descriptor, "rb") as handle:
        opened = os.fstat(handle.fileno())
        require(identity(before) == identity(opened), "input replaced before read")
        raw = handle.read(limit + 1)
        require(len(raw) == before.st_size, "complete bounded input")
        require(identity(opened) == identity(os.fstat(handle.fileno())), "input changed during read")
    require(identity(before) == identity(path.lstat()), "input path changed during read")
    return {"path": str(path), "bytes": raw, "identity": identity(before), "sha256": sha(raw)}


def unchanged(first):
    current = snapshot(first["path"], max(len(first["bytes"]), 1))
    require(current["identity"] == first["identity"] and current["sha256"] == first["sha256"],
            "captured input changed")


def public_pin(value):
    return {"path": value["path"], "bytes": len(value["bytes"]), "sha256": value["sha256"],
            "identity": [str(item) for item in value["identity"]]}


def retain(output, name, raw):
    require(re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", name) is not None, "owned retention filename")
    require(isinstance(raw, bytes) and len(raw) <= MAX_JSON + 1, "bounded retained bytes including first extra")
    with (output / name).open("xb") as handle:
        handle.write(raw)
        handle.flush()
        os.fsync(handle.fileno())


def cargo_inputs(root, lock_bytes):
    manifest = snapshot(root / "Cargo.toml")
    document = tomllib.loads(manifest["bytes"].decode("utf-8"))
    workspace = document["workspace"]
    members = workspace["members"]
    require(isinstance(members, list) and 0 < len(members) <= 64, "bounded explicit workspace members")
    locals_by_name, materials = {}, [manifest]
    for member in members:
        require(isinstance(member, str) and re.fullmatch(r"[A-Za-z0-9_-]+(?:/[A-Za-z0-9_-]+)+", member),
                "explicit in-root member path")
        material = snapshot(root / member / "Cargo.toml")
        materials.append(material)
        package = tomllib.loads(material["bytes"].decode("utf-8"))["package"]
        version = package["version"]
        if version == {"workspace": True}:
            version = workspace["package"]["version"]
        name = package["name"]
        require(name not in locals_by_name and isinstance(version, str), "unique local package identity")
        locals_by_name[name] = version
    packages, local, seen = [], [], set()
    lock = tomllib.loads(lock_bytes.decode("utf-8"))
    require(lock.get("version") == 4 and isinstance(lock.get("package"), list), "Cargo lock v4")
    require(0 < len(lock["package"]) <= 4096, "bounded Cargo package count")
    for row in lock["package"]:
        name, version, source = row["name"], row["version"], row.get("source")
        require(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_-]+", name), "Cargo package name")
        require(isinstance(version, str) and VERSION.fullmatch(version), "Cargo registry semver")
        require((name, version) not in seen, "duplicate Cargo package")
        seen.add((name, version))
        if source is None:
            require(locals_by_name.get(name) == version, "unregistered nonregistry Cargo package")
            local.append({"name": name, "version": version, "scope": "local-not-registry-advisory-audited"})
        else:
            require(source == "registry+https://github.com/rust-lang/crates.io-index", "unsupported Cargo origin")
            require(re.fullmatch(r"[a-f0-9]{64}", row.get("checksum", "")), "registry checksum")
            packages.append({"package": {"ecosystem": "crates.io", "name": name}, "version": version})
    return packages, local, materials
