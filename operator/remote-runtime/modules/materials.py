"""Inert material reconstruction only. Checkout authentication belongs to the caller."""
import gzip, hashlib, http.client, io, json, os, pathlib, stat, sys, tarfile, time, types, urllib.parse, zipfile, zlib

SEED_SHA = 'd08ccfa8a9f817e24cd6087fc45d90fb2a57b60695cabca45dac420ffe5f3759'
MANIFEST_SHA = '2e5f82de9c542614d5ac9e27400baec76f3b39fecdaf88e93c063bb2d181c5b0'
PREFIX = 'm6-remote-runtime-seed'
LIMIT = 100 * 1024**2
PINS = {
 'compiler': (14, 134170551, 134195200, '8dfc6bb5d66e6f06130dd0557fd8e384720bb9c77bb88fbe0e932c892b8f0e8c', 45536929, 'e3c134cc58324c391d6f715d73958af12efe022b5e4cc8b7c6faeaedd51ed839'),
 'evaluator': (419, 548525078, 548915200, 'd01ceb37da64bf1f2c798be95ad1cd6eb51e4a3e4c71878dc5a35bcb1950a27f', 227574493, '4794d1ca22894a62fdbabb2d21c5fc2aa3999915b75f867aa2cd973771b5b0c9')}

def require(value, reason):
    if not value: raise ValueError(reason)

def digest(raw): return hashlib.sha256(raw).hexdigest()

def canonical(value): return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()

def identity(s): return tuple(getattr(s, k) for k in ['st_dev', 'st_ino', 'st_mode', 'st_nlink', 'st_uid', 'st_gid', 'st_size', 'st_mtime_ns', 'st_ctime_ns'])

def safe(name):
    require(type(name) is str and name.isascii() and not name.startswith('/') and
            all(p not in ['', '.', '..'] for p in name.split('/')), 'canonical relative path')

def checked_path(value, directory=False):
    p = pathlib.Path(value)
    require(p.is_absolute() and p.resolve() == p, 'absolute no-symlink path')
    s = p.lstat()
    require((stat.S_ISDIR(s.st_mode) if directory else stat.S_ISREG(s.st_mode)) and
            s.st_uid == 0 and not s.st_mode & 0o7022 and (directory or s.st_nlink == 1), 'root-owned ordinary path')
    return p

def pin(path, expected=None, algorithm='sha256'):
    p = checked_path(path); before = p.stat(); h = hashlib.new(algorithm); n = 0
    with p.open('rb') as source:
        while chunk := source.read(65536): h.update(chunk); n += len(chunk)
    require(identity(before) == identity(p.stat()) and n == before.st_size, 'source changed during read')
    result = {'path': str(p), 'bytes': n, algorithm: h.hexdigest()}
    if expected: require(n == expected['bytes'] and h.hexdigest() == expected[algorithm], 'whole byte identity')
    return result

def save(path, value):
    with path.open('xb') as out: out.write(canonical(value))
    path.chmod(0o400)

def inventory(seed):
    seed = checked_path(seed, True); p = seed / 'MANIFEST.json'
    require(pin(p)['sha256'] == MANIFEST_SHA and stat.S_IMODE(p.stat().st_mode) == 0o444, 'approved seed manifest identity/mode')
    manifest = json.loads(p.read_bytes()); rows = manifest['files']
    require(len(rows) == 775 and len({r['path'] for r in rows}) == 775, 'closed seed inventory')
    expected = {'MANIFEST.json'}
    for row in rows:
        safe(row['path']); require(row['mode'] in [0o444, 0o555], 'seed mode')
        path = seed / row['path']; pin(path, row)
        require(stat.S_IMODE(path.stat().st_mode) == row['mode'], 'exact seed ordinary mode'); expected.add(row['path'])
    expected_dirs = {str(p) for name in expected for p in pathlib.PurePosixPath(name).parents if str(p) != '.'}
    actual = set(); directories = set()
    for path in seed.rglob('*'):
        s = path.lstat(); require(not path.is_symlink(), 'seed link refused')
        if stat.S_ISREG(s.st_mode): actual.add(str(path.relative_to(seed)))
        else:
            checked_path(path, True); require(stat.S_IMODE(s.st_mode) == 0o755, 'seed directory mode'); directories.add(str(path.relative_to(seed)))
    require(actual == expected and directories == expected_dirs, 'no extra or missing seed member')
    return seed

def unpack_seed(archive, destination, expected_archive_sha256, expected_manifest_sha256):
    """Optional closed seed unpacker; neither checkout nor any template is unpacked."""
    require(os.geteuid() == 0 and expected_archive_sha256 == SEED_SHA and
            expected_manifest_sha256 == MANIFEST_SHA, 'explicit approved seed pins/root required')
    require(pin(archive)['sha256'] == SEED_SHA and pathlib.Path(archive).stat().st_size == 32379261, 'whole seed archive')
    target = pathlib.Path(destination); require(target.is_absolute() and not target.exists(), 'fresh seed destination')
    checked_path(target.parent, True)
    with gzip.open(archive, 'rb') as stream: raw = stream.read(LIMIT + 1)
    require(len(raw) <= LIMIT, 'complete expanded seed bound')
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:') as source:
        members = source.getmembers(); require(len(members) <= 2048, 'seed member bound')
        seen = set(); files = {}
        for m in members:
            safe(m.name); require(m.name not in seen and (m.name == PREFIX or m.name.startswith(PREFIX + '/')), 'closed seed prefix')
            seen.add(m.name); require(m.uid == m.gid == m.mtime == 0 and not m.uname and not m.gname and not m.pax_headers, 'canonical USTAR ownership')
            require(m.isdir() or m.isfile(), 'seed links/special members refused')
            require(m.mode == 0o755 if m.isdir() else m.mode in [0o444, 0o555], 'seed member mode')
            if m.isfile(): files[m.name] = m
        require(len(files) == 776 and sum(m.size for m in files.values()) <= LIMIT, 'closed seed file bound')
        manifest_raw = source.extractfile(files[PREFIX + '/MANIFEST.json']).read()
        require(digest(manifest_raw) == MANIFEST_SHA, 'inner manifest identity')
        rows = json.loads(manifest_raw)['files']; expected = {PREFIX + '/' + r['path']: r for r in rows}
        require(len(expected) == 775 and set(files) == {*expected, PREFIX + '/MANIFEST.json'}, 'closed inner inventory')
        directories = {PREFIX}
        for name in files:
            directories.update(str(p) for p in pathlib.PurePosixPath(name).parents if str(p) != '.')
        require(seen == set(files) | directories, 'exact seed directory closure')
        target.mkdir(mode=0o700)
        for name in sorted(directories):
            path = target / name; path.mkdir(mode=0o755, parents=True, exist_ok=True)
        for name, m in files.items():
            data = source.extractfile(m).read(m.size + 1); require(len(data) == m.size, 'seed member length')
            if name in expected:
                row = expected[name]; require(len(data) == row['bytes'] and digest(data) == row['sha256'] and m.mode == row['mode'], 'seed leaf identity')
            path = target / name
            with path.open('xb') as out: out.write(data)
            path.chmod(m.mode)
    require(pin(archive)['sha256'] == SEED_SHA, 'seed archive stability')
    return inventory(target / PREFIX)

def request(url, deadline, maximum, out=None, algorithm='sha256'):
    """GET only; credentials never leave api.github.com, URLs/headers never persisted."""
    allowed = {'api.github.com', 'github.com', 'release-assets.githubusercontent.com', 'storage.googleapis.com', 'registry.npmjs.org'}
    h = hashlib.new(algorithm); n = 0
    for hop in range(5):
        u = urllib.parse.urlsplit(url)
        require(u.scheme == 'https' and u.hostname in allowed and not u.username and not u.password and u.port in [None, 443] and not u.fragment, 'closed HTTPS GET host')
        require(time.monotonic() < deadline, 'whole acquisition deadline')
        headers = {'Accept': 'application/json' if out is None else 'application/octet-stream', 'User-Agent': 'zryna-inert-material-preparer-v1', 'Accept-Encoding': 'identity', 'X-GitHub-Api-Version': '2022-11-28'}
        token = os.environ.get('GITHUB_TOKEN') if u.hostname == 'api.github.com' else None
        if token: headers['Authorization'] = 'Bearer ' + token
        connection = http.client.HTTPSConnection(u.hostname, timeout=min(60, max(1, deadline-time.monotonic())))
        try:
            connection.request('GET', u.path + ('?' + u.query if u.query else ''), headers=headers); response = connection.getresponse()
            if response.status in [301, 302, 303, 307, 308]:
                require(hop < 4 and response.getheader('Location'), 'finite redirect chain')
                url = urllib.parse.urljoin(url, response.getheader('Location')); continue
            require(response.status == 200 and response.getheader('Content-Encoding', 'identity') == 'identity', 'whole GET HTTP ' + str(response.status))
            chunks = []
            while True:
                if response.isclosed(): break
                remaining = deadline - time.monotonic(); require(remaining > 0, 'whole acquisition deadline')
                sock = connection.sock or response.fp.raw._sock; sock.settimeout(min(30, remaining))
                chunk = response.read(65536)
                if not chunk: break
                require(time.monotonic() < deadline, 'whole acquisition deadline'); n += len(chunk); require(n <= maximum, 'first extra acquisition byte')
                h.update(chunk)
                if out is None: chunks.append(chunk)
                else: out.write(chunk)
            return (b''.join(chunks) if out is None else {'bytes': n, algorithm: h.hexdigest()})
        except (OSError, http.client.HTTPException): raise RuntimeError('bounded HTTPS acquisition failed') from None
        finally: connection.close()
    raise ValueError('redirect bound')

def tar_map(path):
    archive = tarfile.open(path, 'r:gz'); members = archive.getmembers(); require(0 < len(members) <= 1024, 'source tar member bound')
    root = members[0].name.split('/')[0]; seen = set()
    for m in members:
        safe(m.name); require(m.name not in seen and (m.name == root or m.name.startswith(root + '/')) and (m.isdir() or m.isfile()), 'ordinary one-root source tar')
        seen.add(m.name)
    return archive, {m.name: m for m in members}, root

def tar_leaf(source, name, row):
    archive, members, root = source; m = members.get(root + '/' + name)
    require(m is not None and m.isfile() and m.size == row['bytes'], 'selected ordinary tar member')
    return archive.extractfile(m).read(row['bytes'] + 1)

def build_materials(repo, seedDirectory, workDirectory):
    require(os.geteuid() == 0 and zlib.ZLIB_RUNTIME_VERSION == '1.3.1', 'root interpreter must start with pinned libz1.3.1')
    repo = checked_path(repo, True); seed = inventory(seedDirectory); work = pathlib.Path(workDirectory)
    require(work.is_absolute() and not work.exists(), 'fresh material work directory'); checked_path(work.parent, True); work.mkdir(mode=0o700)
    deadline = time.monotonic() + 1200; inputs = json.loads((seed / 'metadata/downloads.json').read_bytes())['inputs']
    require(len(inputs) == 3, 'three fixed original inputs'); save(work / 'ORIGINAL_INPUTS.json', inputs)
    release = json.loads(request('https://api.github.com/repos/zryna/zryna/releases/388328382', deadline, 2*1024**2))
    require(release['id'] == 388328382 and release['immutable'] is True and release['draft'] is False and release['tag_name'] == 'v0.2.3', 'fresh original immutable legacy release')
    subjects = [a for a in release['assets'] if a['id'] == 563209159]; require(len(subjects) == 1, 'unique original legacy subject')
    a = subjects[0]; require(a['state'] == 'uploaded' and a['name'] == 'zryna-0.2.3-x86_64-unknown-linux-gnu.tar.gz' and a['size'] == inputs[0]['bytes'] and a['digest'] == 'sha256:' + inputs[0]['sha256'], 'fresh legacy subject identity')
    save(work / 'LEGACY_READONLY_METADATA.json', {'id': release['id'], 'immutable': True, 'draft': False, 'tag': release['tag_name'], 'assetId': a['id'], 'bytes': a['size'], 'sha256': inputs[0]['sha256']})
    acquired = []
    for item in inputs:
        path = work / item['name']; algorithm = 'sha512' if 'sha512' in item else 'sha256'
        with path.open('xb') as out: actual = request(item['url'], deadline, item['bytes'], out, algorithm)
        path.chmod(0o400); require(actual == {'bytes': item['bytes'], algorithm: item[algorithm]}, 'whole immutable download pin'); pin(path, item, algorithm); acquired.append(path)
    legacy = tar_map(acquired[0]); playwright = tar_map(acquired[2]); chrome = zipfile.ZipFile(acquired[1]); infos = chrome.infolist()
    require(len(infos) <= 512 and len({m.filename for m in infos}) == len(infos), 'unique bounded browser ZIP')
    preparation = repo / 'examples/playground/restricted/daemon/preparation'; modulepins = {}
    for name in ['emit', 'archive', 'handles', 'plan']:
        require(name not in sys.modules, 'isolated original emitter module namespace'); modulepins[name] = pin(preparation / (name + '.py'))
    result = {}
    try:
        # Execute only the exact selected checkout source; never read/write .pyc.
        for name in ['plan', 'handles', 'archive', 'emit']:
            source = preparation / (name + '.py'); raw = source.read_bytes()
            require(len(raw) == modulepins[name]['bytes'] and digest(raw) == modulepins[name]['sha256'], 'selected original emitter source')
            module = types.ModuleType(name); module.__file__ = str(source); sys.modules[name] = module
            exec(compile(raw, str(source), 'exec'), module.__dict__)
        emitter = sys.modules['emit']
        for role in ['compiler', 'evaluator']:
            manifest_path = seed / ('materials/' + role + '-manifest.json'); manifest = json.loads(manifest_path.read_bytes()); rows = manifest['files']; p = PINS[role]
            require(len(rows) == p[0] and sum(r['bytes'] for r in rows) == p[1], 'original complete material count/bytes')
            root = work / ('sources-' + role); root.mkdir(mode=0o700)
            for row in rows:
                rel = row['path']; safe(rel); path = root / rel; path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                if rel.startswith('browser/'):
                    member = chrome.getinfo(rel.removeprefix('browser/')); mode = member.external_attr >> 16
                    require(not member.is_dir() and not member.flag_bits & 1 and stat.S_IFMT(mode) in [0, stat.S_IFREG] and member.file_size == row['bytes'], 'ordinary selected browser ZIP member')
                    with chrome.open(member) as stream: raw = stream.read(row['bytes'] + 1)
                else: raw = tar_leaf(playwright if rel.startswith('playwright/') else legacy, rel.removeprefix('playwright/') if rel.startswith('playwright/') else rel, row)
                require(len(raw) == row['bytes'] and digest(raw) == row['sha256'] and row['mode'] in [0o444, 0o555], 'selected manifest leaf identity')
                with path.open('xb') as out: out.write(raw)
                path.chmod(0o500 if row['mode'] == 0o555 else 0o400)
            require({str(x.relative_to(root)) for x in root.rglob('*') if x.is_file()} == {r['path'] for r in rows}, 'no extra material leaves')
            document = {'format': 'zryna.playground-preparation-plan.v1', 'version': 1, 'kind': 'materials', 'role': role, 'directories': manifest['directories'], 'files': [{**r, 'sourcePath': str(root / r['path'])} for r in rows], 'bytes': p[1], 'config': None}
            raw_plan = json.dumps(document, separators=(',', ':'), ensure_ascii=True).encode(); plan = work / (role + '-plan.json')
            with plan.open('xb') as out: out.write(raw_plan)
            plan.chmod(0o400); output = work / (role + '-materials.tar.gz')
            with output.open('xb') as out:
                with gzip.GzipFile(fileobj=out, filename='', mtime=0, mode='wb', compresslevel=6) as compressed: receipt = emitter.run(str(plan), len(raw_plan), digest(raw_plan), compressed)
            output.chmod(0o400); require(receipt['files'] == p[0] and receipt['sourceBytes'] == p[1] and receipt['archiveBytes'] == p[2] and receipt['archiveSha256'] == p[3] and receipt['sourceInventoryBeforeAfterEqual'] is True, 'original deterministic emission receipt')
            material = pin(output, {'bytes': p[4], 'sha256': p[5]}); save(work / (role + '-actual-emission.json'), receipt)
            result[role] = {'manifest': pin(manifest_path), 'sourceDirectory': str(root), 'plan': pin(plan), 'materialArchive': material, 'emissionReceipt': pin(work / (role + '-actual-emission.json'))}
        for name, expected in modulepins.items(): require(pin(preparation / (name + '.py')) == expected, 'checkout emitter source stability')
        for path, item in zip(acquired, inputs): pin(path, item, 'sha512' if 'sha512' in item else 'sha256')
        inventory(seed)
        result.update({'scope': 'Actual inert material bytes only; immutable historical components, no future-source approval, image construction or guest execution.', 'originalEmitterModules': modulepins, 'compressionRuntime': zlib.ZLIB_RUNTIME_VERSION})
        save(work / 'RESULT.json', result); return result
    finally:
        legacy[0].close(); playwright[0].close(); chrome.close()
        for name in ['emit', 'archive', 'handles', 'plan']: sys.modules.pop(name, None)
