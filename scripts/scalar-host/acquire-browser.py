"""Explicit, bounded acquisition only. Produces an unreviewed receipt; never launches a browser."""

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import platform
import stat
import sys
import time
import urllib.request
import zipfile


MAX_ARCHIVE = 256 * 1024 * 1024
MAX_EXPANDED = 1024 * 1024 * 1024
MAX_ENTRIES = 30_000
DEADLINE_SECONDS = 120
REPOSITORY = Path(__file__).resolve().parents[2]


def check_deadline(deadline):
    if time.monotonic() >= deadline:
        raise TimeoutError("browser acquisition exceeded 120 seconds")


class PublisherRedirects(urllib.request.HTTPRedirectHandler):
    def __init__(self, allowed_urls, deadline):
        super().__init__()
        self.allowed_urls = allowed_urls
        self.deadline = deadline

    def verify(self, url):
        check_deadline(self.deadline)
        if url not in self.allowed_urls:
            raise ValueError("browser destination is not a reviewed publisher URL")

    def redirect_request(self, request, response, code, message, headers, new_url):
        self.verify(new_url)
        return super().redirect_request(request, response, code, message, headers, new_url)


def digest(path, deadline):
    check_deadline(deadline)
    value = hashlib.sha256()
    with path.open("rb") as stream:
        while True:
            check_deadline(deadline)
            chunk = stream.read(1024 * 1024)
            check_deadline(deadline)
            if not chunk:
                break
            value.update(chunk)
    check_deadline(deadline)
    return value.hexdigest()


def portable(name):
    parts = PurePosixPath(name).parts
    if not parts or name.startswith("/") or "\\" in name or len(parts) > 32:
        raise ValueError("unsafe archive path")
    for part in parts:
        stem = part.split(".")[0].upper()
        reserved = {"CON", "PRN", "AUX", "NUL"}
        reserved.update(f"{prefix}{n}" for prefix in ("COM", "LPT") for n in range(1, 10))
        if (part in (".", "..") or part.endswith((".", " ")) or stem in reserved
                or any(ord(c) < 32 or c in '<>:"|?*' for c in part)):
            raise ValueError("nonportable archive path")
    if "/".join(parts) != name.rstrip("/"):
        raise ValueError("noncanonical archive path")
    return parts


def main():
    deadline = time.monotonic() + DEADLINE_SECONDS
    if len(sys.argv) != 1:
        raise ValueError("no URL, output, platform or installer overrides are accepted")
    host = {"Linux": "linux", "Windows": "win32"}.get(platform.system())
    if platform.machine().lower() not in ("amd64", "x86_64"):
        raise ValueError("unsupported browser architecture")
    key = f"{host}-x64"
    pin = json.loads((REPOSITORY / "tests/scalar-host/browser-pin.json").read_text())
    selected = pin["platforms"][key]
    redirects = PublisherRedirects({selected["url"], selected["publisherUrl"]}, deadline)
    redirects.verify(selected["url"])
    cache = REPOSITORY / ".zryna/cache"
    cache.mkdir(parents=True, exist_ok=True)
    if cache.is_symlink() or cache.resolve() != cache.absolute():
        raise ValueError("cache must be a direct private repository directory")
    output = cache / f"scalar-browser-{key}-{pin['browserVersion']}"
    output.mkdir()  # create-only; failures deliberately retain a bounded diagnostic receipt
    archive = output / "archive.zip"
    check_deadline(deadline)
    opener = urllib.request.build_opener(redirects)
    with opener.open(selected["url"], timeout=min(10, deadline - time.monotonic())) as response, archive.open("xb") as target:
        final_url = response.geturl()
        redirects.verify(final_url)
        if final_url != selected["publisherUrl"]:
            raise ValueError("final browser destination differs from the reviewed publisher URL")
        advertised = response.headers.get("Content-Length")
        if advertised is not None and int(advertised) > MAX_ARCHIVE:
            raise ValueError("compressed browser exceeds 256 MiB")
        total = 0
        while True:
            check_deadline(deadline)
            chunk = response.read(1024 * 1024)
            check_deadline(deadline)
            if not chunk:
                break
            total += len(chunk)
            if total > MAX_ARCHIVE:
                raise ValueError("compressed browser exceeds 256 MiB")
            target.write(chunk)
            check_deadline(deadline)
    check_deadline(deadline)
    browser = output / "browser"
    browser.mkdir()
    with zipfile.ZipFile(archive) as bundle:
        entries = bundle.infolist()
        check_deadline(deadline)
        if len(entries) > MAX_ENTRIES or sum(item.file_size for item in entries) > MAX_EXPANDED:
            raise ValueError("browser archive exceeds expansion budgets")
        seen = set()
        for item in entries:
            check_deadline(deadline)
            parts = portable(item.filename)
            key = "/".join(parts).casefold()
            if key in seen:
                raise ValueError("duplicate archive identity")
            seen.add(key)
            mode = item.external_attr >> 16
            if stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR) or item.flag_bits & 1:
                raise ValueError("archive links, special entries and encryption are unsupported")
            destination = browser.joinpath(*parts)
            if item.is_dir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            with bundle.open(item) as source, destination.open("xb") as target:
                written = 0
                while chunk := source.read(1024 * 1024):
                    check_deadline(deadline)
                    written += len(chunk)
                    if written > item.file_size:
                        raise ValueError("archive expanded beyond declared size")
                    target.write(chunk)
                    check_deadline(deadline)
            if written != item.file_size:
                raise ValueError("truncated archive entry")
            if os.name != "nt" and mode & 0o111:
                destination.chmod(0o700)
    check_deadline(deadline)
    files = []
    for path in browser.rglob("*"):
        check_deadline(deadline)
        if path.is_file():
            files.append({"path": path.relative_to(browser).as_posix(),
                          "bytes": path.stat().st_size, "sha256": digest(path, deadline)})
        check_deadline(deadline)
    files.sort(key=lambda entry: entry["path"])
    check_deadline(deadline)
    notices = [entry["path"] for entry in files
               if any(word in entry["path"].lower() for word in ("license", "notice", "credits"))
               or entry["path"].endswith("/ABOUT")]
    if not notices or not (browser / selected["executable"]).is_file():
        raise ValueError("browser executable or bundled license inventory missing")
    inventory = json.dumps(files, separators=(",", ":"), ensure_ascii=True).encode()
    check_deadline(deadline)
    (output / "inventory.json").write_bytes(inventory)
    check_deadline(deadline)
    receipt = {"state": "unreviewed-publisher-https-acquisition", "url": selected["url"],
               "finalUrl": final_url, "archiveSha256": digest(archive, deadline),
               "inventorySha256": hashlib.sha256(inventory).hexdigest(),
               "archiveBytes": archive.stat().st_size, "fileCount": len(files),
               "expandedBytes": sum(entry["bytes"] for entry in files), "notices": notices}
    receipt_text = json.dumps(receipt, indent=2)
    check_deadline(deadline)
    (output / "receipt.json").write_text(receipt_text + "\n")
    check_deadline(deadline)
    print(receipt_text)
    check_deadline(deadline)


if __name__ == "__main__":
    main()
