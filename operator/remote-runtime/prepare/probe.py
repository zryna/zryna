"""Fresh import of the explicitly pinned historical trusted probe; no guest start."""
import hashlib,io,json,pathlib,tarfile


def prepare_probe(context,Engine,pin,save):
    p=context['prepare'];root=pathlib.Path(context['runtime']['dependencies']['probe']['directory']);root.mkdir(mode=0o700)
    for name in ['probeExport','probeConfigBlob','probeSource']:pin(p[name])
    archive=pathlib.Path(p['probeExport']['path']);blob=pathlib.Path(p['probeConfigBlob']['path']).read_bytes()
    config=json.loads(blob);identity='sha256:'+hashlib.sha256(blob).hexdigest()
    assert identity=='sha256:2d52b61393de48e65215286ee502e87164748dd7fe3974696552db9bb4748d0e'
    assert p['probeExport']['bytes']==757248 and p['probeExport']['sha256']=='4494dbc15044988fa792f4017bc1339ec69e1b152dbedb1c01b23f357cf39aa4'
    assert p['probeSource']['bytes']==4022 and p['probeSource']['sha256']=='2ded19c1ef96935a78f7c63690ef5849833a5eacee5ff951e9aa28d2824472ff'
    assert config['architecture']=='amd64' and config['os']=='linux'
    assert config['config']=={'User':'65534:65534','Env':['PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin'],'Entrypoint':['/probe'],'WorkingDir':'/'}
    with tarfile.open(archive,mode='r:') as source:
        members=source.getmembers();assert len(members)<=64 and sum(m.size for m in members)<=2097152
        assert len({m.name for m in members})==len(members)
        for m in members:
            assert m.isdir() or m.isfile()
            assert not m.name.startswith('/') and all(v not in ['','..','.'] for v in m.name.rstrip('/').split('/'))
            if m.isfile() and m.name.startswith('blobs/sha256/'):
                assert hashlib.sha256(source.extractfile(m).read()).hexdigest()==m.name.removeprefix('blobs/sha256/')
        manifest=json.loads(source.extractfile('manifest.json').read());assert len(manifest)==1
        selected=manifest[0];assert selected['Config']=='blobs/sha256/'+identity[7:]
        assert source.extractfile(selected['Config']).read()==blob and selected['RepoTags'] in [None,[]]
        assert ['sha256:'+hashlib.sha256(source.extractfile(name).read()).hexdigest() for name in selected['Layers']]==config['rootfs']['diff_ids']
    def emit(out):
        with archive.open('rb') as stream:
            while chunk:=stream.read(65536):out.write(chunk)
        return p['probeExport']
    engine=Engine(65534,seconds=120)
    raw,_=engine.request('POST','/images/load?quiet=0',emitter=emit,stream_maximum=p['probeExport']['bytes'],statuses=(200,))
    with (root/'LOAD.stdout').open('xb') as stream:stream.write(raw)
    events=[json.loads(line) for line in raw.splitlines() if line];assert events and all(type(v) is dict and not any(k in v for k in ['error','errorDetail']) for v in events)
    state=engine.json('GET','/images/'+identity+'/json');save(root/'IMAGE.json',[state])
    assert state['Id']==identity and state['Os']=='linux' and state['Architecture']=='amd64'
    assert state.get('Variant','')==state.get('OsVersion','')=='' and state['RootFS']=={'Type':'layers','Layers':config['rootfs']['diff_ids']}
    expected={**config['config'],'Cmd':None,'Labels':None,'OnBuild':None,'Volumes':None}
    defaults={'AttachStderr':False,'AttachStdin':False,'AttachStdout':False,'Domainname':'','Hostname':'','Image':'','OpenStdin':False,'StdinOnce':False,'Tty':False}
    actual=state['Config'];assert all(actual.get(k)==v for k,v in expected.items())
    assert all(k in expected or k in defaults and v==defaults[k] for k,v in actual.items())
    with (root/'EXACT_IMAGE_CONFIG_BLOB.json').open('xb') as stream:stream.write(blob)
    source_bytes=pathlib.Path(p['probeSource']['path']).read_bytes()
    assert len(source_bytes)==p['probeSource']['bytes'] and hashlib.sha256(source_bytes).hexdigest()==p['probeSource']['sha256']
    with (root/'probe.c').open('xb') as stream:stream.write(source_bytes)
    (root/'probe.c').chmod(0o400)
    for name in ['probeExport','probeConfigBlob','probeSource']:pin(p[name])
    return {'scope':'Unchanged explicitly selected historical trusted probe tool, freshly imported and inspected; no guest execution/new-source image claim.','image':identity,'inspect':str(root/'IMAGE.json'),'configBlob':str(root/'EXACT_IMAGE_CONFIG_BLOB.json'),'source':p['probeSource'],'export':p['probeExport']}
