"""Reviewed operator Git blobs -> rooted inputs -> one owned host; no approval authority."""
import datetime, gzip, hashlib, io, json, os, pathlib, re, shutil, signal, stat, subprocess, sys, tarfile, time

BASE = pathlib.Path('/workspace/scratch')
MAX_CAPTURE = 1024**3
HOST_FILES = {'HOST_INTENT.json','HOST_RESULT.json','OBSERVER_INTENT.json','OBSERVER_RESULT.json','PREPARED.json','REMOTE_PREPARED.json','broker-reviewed.py','containerd.log','dockerd.log','broker.stdout','broker.stderr','containerd.toml','daemon.json','dockerd.pid'} | {prefix+suffix for prefix in ['engine-acquisition','engine-info','observer','prepare','approval','runtime','mount-private','cgroup-unmount','cgroup-readonly','socket-overlay'] for suffix in ['.intent.json','.started.json','.json','.stdout','.stderr']}
def need(value, reason):
    if not value: raise ValueError(reason)
def sha(raw): return hashlib.sha256(raw).hexdigest()
def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def save(path, value):
    with path.open('x') as out: json.dump(value, out, sort_keys=True, indent=2); out.write('\n')
    path.chmod(0o444)
def rel(value):
    need(isinstance(value, str) and value.isascii() and not value.startswith('/') and all(p not in ['', '.', '..'] for p in value.split('/')), 'canonical relative path')
    return value
def pin(path, empty=False):
    path = pathlib.Path(path); s = path.lstat()
    need(path.resolve() == path and stat.S_ISREG(s.st_mode) and s.st_nlink == 1 and s.st_uid == 0 and not s.st_mode & 0o7022 and (empty or s.st_size > 0), 'rooted ordinary single-link file')
    h = hashlib.sha256()
    with path.open('rb') as stream:
        while b := stream.read(65536): h.update(b)
    a = path.stat(); fields = ['st_dev', 'st_ino', 'st_size', 'st_mode', 'st_uid', 'st_gid', 'st_nlink', 'st_mtime_ns', 'st_ctime_ns']
    need(all(getattr(s, k) == getattr(a, k) for k in fields), 'stable file')
    return {'path': str(path), 'bytes': s.st_size, 'sha256': h.hexdigest()}
def git(repo, *args):
    env = {'PATH': '/usr/bin:/bin', 'HOME': '/nonexistent', 'LC_ALL': 'C', 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_NO_REPLACE_OBJECTS': '1', 'GIT_TERMINAL_PROMPT': '0'}
    return subprocess.run(['/usr/bin/git', '-c', 'safe.directory=' + str(repo), '--no-replace-objects', '-C', str(repo), *args], env=env, capture_output=True, timeout=30, check=True).stdout
def blob(repo, commit, path):
    rel(path); value = git(repo, 'ls-tree', '-z', commit, '--', path).split(b'\0')
    need(len(value) == 2 and value[1] == b'', 'one selected operator blob')
    metadata, name = value[0].split(b'\t'); mode, kind, oid = metadata.split()
    need(name.decode() == path and mode in [b'100644', b'100755'] and kind == b'blob', 'ordinary Git blob')
    raw = git(repo, 'cat-file', 'blob', oid.decode())
    need(hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() == oid.decode(), 'actual Git object identity')
    return raw
def rooted_directory(path, mode=0o755):
    path.mkdir(mode=mode)
    need(path.resolve() == path and path.stat().st_uid == 0 and not path.stat().st_mode & 0o022, 'rooted directory')
def load_selected(path, name):
    # This file has already been authenticated as an exact operator Git blob.
    raw = path.read_bytes(); module = type(sys)(name); module.__file__ = str(path)
    exec(compile(raw, str(path), 'exec'), module.__dict__); return module
def expand(value, tokens):
    if isinstance(value, str):
        for key, replacement in tokens.items(): value = value.replace(key, replacement)
        need(not re.search(r'@[A-Z_]+@', value), 'unbound template token'); return value
    if isinstance(value, list): return [expand(v, tokens) for v in value]
    if isinstance(value, dict): return {k: expand(v, tokens) for k, v in value.items()}
    return value
def unpack_templates(archive, manifest, target):
    need(pin(archive)['sha256'] == 'd0e00bec690a878d3c88c0115ef88d2819782416b2e319f6c4f5896d7f3c5087' and archive.stat().st_size == 412552, 'approved historical template archive')
    need(pin(manifest)['sha256'] == '0b06b320a37b9d0048c97ee2684a1ae3f2c0a0c79d8813f23908fda8ddc7351c', 'approved external template manifest')
    catalogue=json.loads(manifest.read_bytes()); rows=catalogue['files']; prefix='m6-410-runtime-templates'
    need(catalogue['newSourceEvidence'] is False and catalogue['approvalGranted'] is False and len(rows)==150, 'historical inert templates only')
    expected={prefix+'/'+rel(r['carrierPath']):r for r in rows}; need(len(expected)==150,'closed template leaves')
    directories={prefix}|{str(p) for name in expected for p in pathlib.PurePosixPath(name).parents if str(p)!='.'}
    with gzip.open(archive,'rb')as stream: raw=stream.read(32*1024**2+1)
    need(len(raw)==3845120,'whole expanded template archive bound')
    with tarfile.open(fileobj=io.BytesIO(raw),mode='r:')as source:
        members=source.getmembers(); need(len(members)==173 and len({m.name for m in members})==173,'closed unique template members')
        need({m.name for m in members}==set(expected)|directories,'exact template member set')
        rooted_directory(target)
        for name in sorted(directories): (target/name).mkdir(mode=0o755,parents=True,exist_ok=True)
        for m in members:
            rel(m.name); need(m.uid==m.gid==m.mtime==0 and not m.uname and not m.gname and not m.pax_headers,'canonical template ownership')
            if m.name in directories: need(m.isdir()and m.mode==0o755 and m.size==0,'ordinary template directory'); continue
            row=expected[m.name]; need(m.isfile()and m.mode==0o644 and m.size==row['bytes'],'ordinary template leaf')
            data=source.extractfile(m).read(m.size+1); need(len(data)==row['bytes']and sha(data)==row['sha256'],'whole original template/oracle identity')
            path=target/m.name
            with path.open('xb')as out:out.write(data)
            path.chmod(0o444)
    return target/prefix
def capture(source, target, recursive, budget, selected=None):
    need(source.is_dir() and source.resolve() == source, 'selected evidence root')
    rooted_directory(target)
    paths = sorted(source.rglob('*') if recursive else source.iterdir()); rows = []
    for p in paths:
        if selected is not None and p.name not in selected: continue
        s = p.lstat()
        if stat.S_ISDIR(s.st_mode): continue
        if not stat.S_ISREG(s.st_mode):
            rows.append({'selectedPath':str(p),'notRead':True,'specialMode':s.st_mode,'ownerUid':s.st_uid}); continue
        need(s.st_size <= MAX_CAPTURE-budget[0], 'capture file first extra byte bound')
        descriptor = pin(p, True); budget[0] += descriptor['bytes']; budget[1] += 1
        need(budget[0] <= MAX_CAPTURE and budget[1] <= 10000, 'closed capture bound')
        destination = target / p.relative_to(source); destination.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        with p.open('rb') as inp, destination.open('xb') as out: shutil.copyfileobj(inp, out, 65536)
        destination.chmod(0o444); got = pin(destination, True)
        need((got['bytes'], got['sha256']) == (descriptor['bytes'], descriptor['sha256']) and pin(p, True) == descriptor, 'whole captured source identity')
        rows.append({'relativePath': str(p.relative_to(source)), 'original': descriptor, 'captured': got})
    return rows

def bootstrap():
    need(os.geteuid() == 0 and len(sys.argv) == 11, 'root bootstrap arguments')
    _, _, operator, source_checkout, operator_commit, bundle_sha, source_commit, source_tree, review_sha, compiler_sha, node_path = sys.argv
    for value, length in [(operator_commit, 40), (source_commit, 40), (source_tree, 40), (bundle_sha, 64), (review_sha, 64), (compiler_sha, 64)]: need(re.fullmatch('[a-f0-9]{'+str(length)+'}', value), 'explicit actual selector')
    run, attempt = os.environ['GITHUB_RUN_ID'], os.environ['GITHUB_RUN_ATTEMPT']
    need(re.fullmatch('[1-9][0-9]*', run) and re.fullmatch('[1-9][0-9]*', attempt) and os.environ['GITHUB_SHA'] == operator_commit and operator_commit != source_commit, 'actual operator/run/source distinction')
    operator = pathlib.Path(operator); source_checkout = pathlib.Path(source_checkout)
    need(git(operator, 'rev-parse', 'HEAD').decode().strip() == operator_commit, 'actual operator checkout')
    raw = blob(operator, operator_commit, 'operator/remote-runtime/bundle.json'); need(sha(raw) == bundle_sha, 'independent bundle pin'); bundle = json.loads(raw)
    need(bundle['format'] == 'zryna.m6-remote-stage-inputs.v1' and bundle['source'] == {'commit': source_commit, 'tree': source_tree}, 'actual source selection')
    need(not pathlib.Path('/workspace/zryna').exists(), 'fresh rooted source target')
    pathlib.Path('/workspace').mkdir(mode=0o755, exist_ok=True)
    need(pathlib.Path('/workspace').stat().st_uid == 0 and not pathlib.Path('/workspace').stat().st_mode & 0o022, 'rooted workspace')
    BASE.mkdir(mode=0o755, exist_ok=True); need(BASE.resolve() == BASE and BASE.stat().st_uid == 0 and not BASE.stat().st_mode & 0o022, 'rooted scratch')
    root = BASE / ('M6_410_REMOTE_OPERATOR_' + run + '_' + attempt); rooted_directory(root)
    with (root/'bundle.json').open('xb') as out: out.write(raw)
    (root/'bundle.json').chmod(0o444)
    for selector, expected in [('HEAD', source_commit), ('HEAD^{tree}', source_tree)]: need(git(source_checkout, 'rev-parse', selector).decode().strip() == expected, 'actual source checkout identity')
    need(git(source_checkout, 'status', '--porcelain', '--untracked-files=all') == b'', 'clean actual source checkout')
    need(git(source_checkout, 'ls-files', '--others', '--ignored', '--exclude-standard') == b'', 'no ignored execution inputs in fresh source')
    shutil.copytree(source_checkout, '/workspace/zryna', symlinks=True)
    for p in [pathlib.Path('/workspace/zryna'), *pathlib.Path('/workspace/zryna').rglob('*')]:
        need(not p.is_symlink(), 'source symlink requires separate review'); os.chown(p, 0, 0); p.chmod(stat.S_IMODE(p.stat().st_mode) & ~0o022)
    need(git('/workspace/zryna', 'rev-parse', 'HEAD').decode().strip() == source_commit and git('/workspace/zryna', 'rev-parse', 'HEAD^{tree}').decode().strip() == source_tree and git('/workspace/zryna', 'status', '--porcelain', '--untracked-files=all') == b'', 'rooted exact source bytes')
    rows = bundle['files']; need(1 <= len(rows) <= 64 and len({r['destination'] for r in rows}) == len(rows), 'closed operator input set')
    installed = {}
    for row in rows:
        rel(row['path']); destination = root / rel(row['destination']); need(row['path'].startswith('operator/remote-runtime/') and row['mode'] in [0o444, 0o555], 'closed operator file/mode')
        data = blob(operator, operator_commit, row['path']); need(len(data) == row['bytes'] and sha(data) == row['sha256'] and len(data) <= 100*1024**2, 'actual operator blob pin')
        destination.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
        with destination.open('xb') as out: out.write(data)
        destination.chmod(row['mode']); installed[row['destination']] = pin(destination)
    need('stage.py' in installed and pathlib.Path(__file__).read_bytes() == (root/'stage.py').read_bytes(), 'same authenticated bootstrap source')
    tools = root / 'bin'; rooted_directory(tools)
    for name, original in [('node', pathlib.Path(node_path).resolve())]:
        s = original.lstat(); need(stat.S_ISREG(s.st_mode) and s.st_nlink == 1, 'physical native interpreter')
        before = original.read_bytes(); destination = tools/name
        with destination.open('xb') as out: out.write(before)
        destination.chmod(0o555); need(original.read_bytes() == before, 'native interpreter source stability'); installed[name] = pin(destination)
    version = subprocess.run([str(tools/'node'), '--version'], capture_output=True, timeout=10, check=True).stdout
    need(version == b'v22.22.1\n', 'actual Node22.22.1')
    material_module = root/rel(bundle['roles']['materialsModule']); material = load_selected(material_module, 'selected_materials')
    seed_wrapper = root/'seed'; seed = material.unpack_seed(root/rel(bundle['roles']['seedArchive']), seed_wrapper, material.SEED_SHA, material.MANIFEST_SHA)
    seed_wrapper.chmod(0o755)  # Only validated inert wrapper: broker UID65534 needs traversal.
    installed['python3.12'] = pin(seed/'host-python/bin/python3.12')
    template_manifest = root/rel(bundle['roles']['templateManifest'])
    templates = unpack_templates(root/rel(bundle['roles']['templateArchive']),template_manifest,root/'templates')
    host_root = BASE / ('M6_410_EPHEMERAL_HOST_' + run + '_' + attempt)
    tokens = {'@OPERATOR@': str(root), '@HOST@': str(host_root), '@SEED@': str(seed), '@TEMPLATES@':str(templates), '@SOURCE@': '/workspace/zryna', '@RUN_ID@': run, '@RUN_ATTEMPT@': attempt, '@OPERATOR_COMMIT@': operator_commit}
    context = expand(bundle['contextTemplate'], tokens)
    need(context['source'] == {'path': '/workspace/zryna', 'commit': source_commit, 'tree': source_tree}, 'bound context source')
    need(context['seed']['directory'] == str(seed) and context['seed']['manifest'] == pin(seed/'MANIFEST.json'), 'actual bound seed input')
    need(context['runtime']['template'] == {'directory':str(templates),'manifest':pin(template_manifest)}, 'actual unchanged historical template input')
    for field in ['frozenMain', 'candidateReview']: need(pin(context[field]['path']) == context[field], 'actual selected context input')
    need(context['candidateReview']['sha256'] == review_sha, 'explicit independent candidate review pin')
    candidate = json.loads(pathlib.Path(context['candidateReview']['path']).read_bytes())
    need(candidate['status'] == 'official-two-replica-candidate-acquired-and-reassembled' and candidate['source'] == {'commit':source_commit,'tree':source_tree} and candidate['compiler']['sha256'] == compiler_sha, 'actual candidate selection')
    binary = pin(root/rel(bundle['roles']['candidateBinary'])); need(binary['bytes'] == candidate['compiler']['bytes'] and binary['sha256'] == compiler_sha, 'actual candidate whole bytes')
    remote_candidate = {**candidate, 'compilerPath':binary['path'], 'originalCompilerPath':candidate['compilerPath'], 'originalIndependentReview':context['candidateReview'], 'pathProjectionMeaning':'Original authenticated candidate bytes; only remote physical compiler path is relocated.'}
    save(root/'candidate-review.remote.json', remote_candidate); context['candidateReview'] = pin(root/'candidate-review.remote.json')
    frozen = json.loads(pathlib.Path(context['frozenMain']['path']).read_bytes()); need(frozen['commit'] == source_commit and frozen['tree'] == source_tree, 'actual frozen source input')
    candidate_root = pathlib.Path(context['prepare']['root'])
    need(candidate_root.is_relative_to(host_root) and candidate_root != host_root and not candidate_root.exists() and context['prepare']['workRoot'] == str(host_root/'preparation-work') and not candidate_root.is_relative_to(host_root/'preparation-work') and not (host_root/'preparation-work').is_relative_to(candidate_root), 'fresh separate candidate/material namespaces')
    save(root/'context.json', context)
    files = {key: pin(root/rel(name)) for key, name in bundle['roles'].items() if key in ['host','observer','common','engineAcquire','broker','prepare','waitApproval','runtime']}
    files.update(python=installed['python3.12'], node=installed['node'], context=pin(root/'context.json'))
    approval = expand(bundle['approvalTemplate'], tokens)
    need(approval['operatorCommit'] == operator_commit and approval['source'] == {'commit':source_commit,'tree':source_tree} and approval['run'] == {'id':run,'attempt':attempt}, 'bound independent approval selectors')
    approval['run'] = {'id':int(run),'attempt':int(attempt)}
    need(approval['root'] == str(candidate_root), 'same candidate approval root')
    limits = bundle['limits']; need(limits == {'prepareSeconds': 2400, 'approvalSeconds': 600, 'runtimeSeconds': 6600}, 'reviewed outer phase bounds')
    manifest = {'format': 'zryna.m6-remote-host-harness.v1', 'run': {'id':run,'attempt':attempt}, 'source':context['source'], 'hostRoot':str(host_root), 'files':files, 'limits':limits, 'approval':approval}
    save(root/'manifest.json', manifest); manifest_pin = pin(root/'manifest.json')
    final_roots = expand(bundle['finalEvidenceRoots'], tokens)
    need(len(final_roots) <= 16 and len({r['name'] for r in final_roots}) == len(final_roots), 'closed final evidence set')
    for item in final_roots:
        path = pathlib.Path(item['path']); need(set(item) == {'name','path'} and path.is_relative_to(BASE) and rel(item['name']) and item['name'] not in ['host','owner'] and path not in [host_root,root] and not path.is_relative_to(host_root/'preparation-work'), 'closed final evidence root')
    configuration = {'root':str(root),'hostRoot':str(host_root),'manifest':manifest_pin,'finalEvidenceRoots':final_roots,'candidateRoot':context['prepare']['root'],'limits':limits,'operatorCommit':operator_commit,'source':context['source'],'installed':installed}
    save(root/'STAGED.json', configuration)
    environment = dict(os.environ); environment.update(LD_LIBRARY_PATH=str(seed/'compression/lib'), PYTHONDONTWRITEBYTECODE='1', PATH=str(root/'bin')+':/usr/sbin:/usr/bin:/sbin:/bin')
    with (root/'owner.stdout').open('xb') as out, (root/'owner.stderr').open('xb') as err:
        owner = subprocess.Popen([installed['python3.12']['path'], '-I', '-B', str(root/'stage.py'), 'own', str(root/'STAGED.json')], env=environment, stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
    epoch = pathlib.Path('/proc/'+str(owner.pid)+'/stat').read_text().rsplit(')',1)[1].split()[19]
    save(root/'OWNER_STARTED.json', {'pid':owner.pid,'epoch':epoch,'startedAt':now(),'stage':pin(root/'stage.py'),'manifest':manifest_pin})
    print(str(root))

def own():
    need(os.geteuid() == 0 and len(sys.argv) == 3, 'root owner arguments'); cfg = json.loads(pathlib.Path(sys.argv[2]).read_bytes()); root = pathlib.Path(cfg['root']); host_root = pathlib.Path(cfg['hostRoot']); manifest = json.loads(pathlib.Path(cfg['manifest']['path']).read_bytes())
    need(pin(cfg['manifest']['path']) == cfg['manifest'], 'actual staged manifest')
    argv = [manifest['files']['python']['path'], '-I', '-B', manifest['files']['host']['path'], cfg['manifest']['path'], cfg['manifest']['sha256']]
    result = {'startedAt':now(),'argv':argv,'approval':False,'acceptance':False}; process = None; budget = [0,0]
    try:
        with (root/'host.stdout').open('xb') as out, (root/'host.stderr').open('xb') as err:
            started = time.monotonic(); process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=out, stderr=err, start_new_session=True)
            result['pid'] = process.pid; save(root/'HOST_STARTED.json',result)
            until = started+cfg['limits']['prepareSeconds']+300
            while not (host_root/'PREPARED.json').exists():
                need(os.waitid(os.P_PID,process.pid,os.WEXITED|os.WNOHANG|os.WNOWAIT) is None, 'host exited before preparation marker')
                need(time.monotonic()<until, 'finite preparation marker wait'); time.sleep(1)
            prepared = json.loads((host_root/'PREPARED.json').read_bytes()); need(prepared['approval'] is False and prepared['acceptance'] is False and prepared['manifest'] == cfg['manifest'], 'actual host preparation-only marker')
            mid = root/'mid-artifact'; rooted_directory(mid); captures = {}
            captures['candidate'] = capture(pathlib.Path(cfg['candidateRoot']), mid/'candidate', True, budget)
            completed = {'PREPARED.json','REMOTE_PREPARED.json','prepare.intent.json','prepare.started.json','prepare.json','prepare.stdout','prepare.stderr'}
            captures['host'] = capture(host_root, mid/'host', False, budget, completed)
            for name in ['manifest.json','context.json','bundle.json','candidate-review.remote.json','STAGED.json','HOST_STARTED.json']:
                data=(root/name).read_bytes(); (mid/name).write_bytes(data); (mid/name).chmod(0o444)
            selected=mid/'selected-inputs'; rooted_directory(selected)
            context=json.loads((root/'context.json').read_bytes()); projection=json.loads((root/'candidate-review.remote.json').read_bytes())
            for name,descriptor in [('candidate-review.original.json',projection['originalIndependentReview']),('FROZEN_MAIN.json',context['frozenMain'])]:
                need(pin(descriptor['path'])==descriptor,'original selected input remains unchanged'); data=pathlib.Path(descriptor['path']).read_bytes()
                with (selected/name).open('xb')as stream:stream.write(data)
                (selected/name).chmod(0o444)
                copied=pin(selected/name); need((copied['bytes'],copied['sha256'])==(descriptor['bytes'],descriptor['sha256']) and pin(descriptor['path'])==descriptor,'whole selected input capture')
            save(mid/'CAPTURE.json',{'meaning':'Actual completed preparation files and whole capture pins; approval and acceptance remain false.','files':captures,'manifest':cfg['manifest']})
            save(root/'MID_READY.json',{'finishedAt':now(),'artifactRoot':str(mid),'capture':pin(mid/'CAPTURE.json'),'approval':False})
            result['returncode'] = process.wait(timeout=max(1,sum(cfg['limits'].values())+360-(time.monotonic()-started)))
            need(result['returncode']==0,'actual host exit nonzero')
    except BaseException as error:
        result['failure']={'type':type(error).__name__,'message':str(error)[:2000]}
    finally:
        if process is not None and process.returncode is None:
            process.send_signal(signal.SIGINT)
            try: process.wait(timeout=90)
            except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=10)
        if process is not None: result['returncode']=process.returncode
        final=root/'final-artifact'; rooted_directory(final); captures={}
        try:
            if host_root.exists(): captures['host']=capture(host_root,final/'host',False,budget,HOST_FILES)
            for row in cfg['finalEvidenceRoots']:
                path=pathlib.Path(row['path'])
                if path.exists(): captures[row['name']]=capture(path,final/rel(row['name']),True,budget)
                else: captures[row['name']]={'missing':True,'selectedPath':str(path)}
            if (root/'mid-artifact/selected-inputs').exists(): captures['selected-inputs']=capture(root/'mid-artifact/selected-inputs',final/'selected-inputs',True,budget)
            captures['owner']=capture(root,final/'owner',False,budget)
            save(final/'CAPTURE.json',{'files':captures,'scope':'Whole retained raw host/phase evidence, never an acceptance verdict.'})
        except BaseException as error: result['captureFailure']={'type':type(error).__name__,'message':str(error)[:2000]}
        result['finishedAt']=now(); save(root/'COMPLETE.json',result); save(final/'COMPLETE.json',result)
    return 0 if result.get('returncode')==0 and not result.get('failure') and not result.get('captureFailure') else 1

if __name__ == '__main__':
    if len(sys.argv)>1 and sys.argv[1]=='bootstrap': bootstrap()
    elif len(sys.argv)>1 and sys.argv[1]=='own': sys.exit(own())
    else: raise ValueError('explicit bootstrap or own command required')
