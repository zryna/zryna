"""Closed data/locator staging only; original Node authentication remains authoritative."""
import json, pathlib, time

COSIGN_URL = 'https://github.com/sigstore/cosign/releases/download/v3.0.5/cosign-linux-amd64'
COSIGN_BYTES = 135112256
COSIGN_SHA = 'db15cc99e6e4837daabab023742aaddc3841ce57f193d11b7c3e06c8003642b2'
ROOT_BYTES = 6787
ROOT_SHA = '6494e21ea73fa7ee769f85f57d5a3e6a08725eae1e38c755fc3517c9e6bc0b66'
NAMES = ['envelope','bundle','archive','policy','verifierPolicy','trustedRoot']
ROLES = dict(zip(NAMES,['publicationEnvelope','publicationBundle','publicationArchive','publicationPolicy','publicationVerifierPolicy','publicationTrustedRoot']))

def need(value, reason):
    if not value: raise ValueError(reason)

def save(path, value):
    with path.open('x')as out:json.dump(value,out,sort_keys=True,indent=2);out.write('\n')
    path.chmod(0o444)

def bind_public_inputs(root, bundle, context, selected_publication_sha, selected_documentation_sha, material):
    root=pathlib.Path(root)
    def selected_path(role):
        name=bundle['roles'][role];need(not name.startswith('/')and all(v not in ['', '.', '..']for v in name.split('/')),'closed public input role')
        return root/name
    def selected_file(role):
        path=selected_path(role);return path,material.pin(path)
    original_path, original_pin=selected_file('publicationSelection')
    need(original_pin==context['publication']['selection']and original_pin['sha256']==selected_publication_sha,'independently selected original publication record')
    original=json.loads(original_path.read_bytes());source={k:context['source'][k]for k in ['commit','tree']}
    need(original['source']==source and original['status']=='actual-immutable-publication-inputs-independently-approved'and set(original['inputs'])==set(NAMES),'actual independently selected six-input publication')
    identity=bundle['publicationIdentity']
    need(set(identity)=={'tag','source','compiler','archive'}and identity['tag']=='v0.1.0-playground-compiler.3'and identity['source']==source,'explicit actual .3 source identity')
    candidate=json.loads(pathlib.Path(context['candidateReview']['path']).read_bytes())
    need(identity['compiler']==candidate['compiler'],'same actual selected candidate compiler')
    acquired=material.pin(context['publication']['acquisition']['path'])
    need(acquired==context['publication']['acquisition'],'whole original actual .3 acquisition proof')
    inputs={};original_pins={}
    for name in NAMES:
        path,actual=selected_file(ROLES[name]);expected=original['inputs'][name]
        need(actual['bytes']==expected['bytes']and actual['sha256']==expected['sha256'],'whole original publication input: '+name)
        inputs[name]={**actual,'ownerUid':0};original_pins[name]=actual
    need(identity['archive']=={k:inputs['archive'][k]for k in ['bytes','sha256']},'actual selected whole .3 archive')
    need((inputs['trustedRoot']['bytes'],inputs['trustedRoot']['sha256'])==(ROOT_BYTES,ROOT_SHA),'original pinned trusted root')
    policy_path=pathlib.Path(inputs['verifierPolicy']['path']);policy=json.loads(policy_path.read_bytes())
    need(set(policy)=={'version','path','bytes','sha256','ownerUid','trustedRootBytes','trustedRootSha256'},'closed original verifier policy')
    need(policy['version']=='3.0.5' and (policy['bytes'],policy['sha256'],policy['trustedRootBytes'],policy['trustedRootSha256'])==(COSIGN_BYTES,COSIGN_SHA,ROOT_BYTES,ROOT_SHA),'unchanged original verifier/crypto pins')
    verifier=root/'bin/cosign-v3.0.5';deadline=time.monotonic()+180
    with verifier.open('xb')as out:download=material.request(COSIGN_URL,deadline,COSIGN_BYTES,out)
    verifier.chmod(0o555);need(download=={'bytes':COSIGN_BYTES,'sha256':COSIGN_SHA},'whole fixed public cosign download')
    verifier_pin=material.pin(verifier,{'bytes':COSIGN_BYTES,'sha256':COSIGN_SHA})
    download_path=root/'COSIGN_ACQUISITION.json';save(download_path,{'fixedVersionedUrl':COSIGN_URL,'versionBoundByApprovedWholeHash':'3.0.5','actualWholeDownload':verifier_pin,'mode':0o555,'ownerUid':0,'wholeBytesVerified':True,'copiedIntoApprovalInventory':False,'scope':'Original fixed verifier tool only. Complete HTTP body was written and whole-hashed; no --version execution, cryptographic verification or runtime approval is claimed by transport.'})
    projected_policy={**policy,'path':str(verifier),'ownerUid':0};remote_policy=root/'verifier-policy.remote.json';save(remote_policy,projected_policy)
    inputs['verifierPolicy']={**material.pin(remote_policy),'ownerUid':0}
    selected={'format':'zryna.m6-remote-publication-selection.v1','version':1,'status':original['status'],'source':source,'originalSelection':original_pin,'originalVerifierPolicy':original_pins['verifierPolicy'],'inputs':inputs}
    publication_selection_path=root/'publication-selection.remote.json';save(publication_selection_path,selected)
    context['publication']['selection']=material.pin(publication_selection_path);context['publication']['verifier']=verifier_pin
    docs_path,docs_pin=selected_file('documentationSelection')
    need(docs_pin==context['documentation']['selection']and docs_pin['sha256']==selected_documentation_sha,'independently selected documentation record')
    docs=json.loads(docs_path.read_bytes())
    need(set(docs)=={'format','version','source','authentication','production'}and docs['format']=='zryna.m6-remote-documentation-selection.v1'and docs['version']==1 and docs['source']==source,'actual same-source documentation selector')
    relocated={**docs}
    for name,role in [('authentication','documentationAuthentication'),('production','documentationProduction')]:
        path,actual=selected_file(role);expected=docs[name]
        need((actual['bytes'],actual['sha256'])==(expected['bytes'],expected['sha256']),'whole original documentation proof: '+name)
        relocated[name]=actual
    remote_docs=root/'documentation-selection.remote.json';save(remote_docs,relocated);context['documentation']['selection']=material.pin(remote_docs)
    public_template=selected_path('publicTemplateDirectory');manifest_path=selected_path('publicTemplateManifest')
    manifest_pin=material.pin(manifest_path);catalogue=json.loads(manifest_path.read_bytes())
    need(catalogue['format']=='zryna.m6-remote-public-template.v1'and catalogue['version']==1 and catalogue['actualFutureSource']is None and catalogue['acceptance']is False and len(catalogue['files'])==6,'inert historical original public template only')
    seen=set()
    for row in catalogue['files']:
        path=row['path'];need(path not in seen and not path.startswith('/')and all(v not in ['', '.', '..']for v in path.split('/')),'closed six public template leaves');seen.add(path)
        actual=material.pin(public_template/path);need((actual['bytes'],actual['sha256'])==(row['bytes'],row['sha256']),'original whole public template bytes')
    actual_files={str(p.relative_to(public_template))for p in public_template.rglob('*')if p.is_file()}
    relative_manifest=str(manifest_path.relative_to(public_template))if manifest_path.is_relative_to(public_template)else None
    need(actual_files==seen|({relative_manifest}if relative_manifest else set()),'no extra public template leaf')
    need(context['runtime']['publicTemplate']=={'directory':str(public_template),'manifest':manifest_pin},'actual public template context')
    artifacts=[{'name':'publication-selection.original.json','pin':original_pin},{'name':'publication-acquisition.original.json','pin':acquired},{'name':'publication-selection.remote.json','pin':context['publication']['selection']},{'name':'verifier-policy.remote.json','pin':material.pin(remote_policy)},{'name':'cosign-acquisition.json','pin':material.pin(download_path)},{'name':'documentation-selection.original.json','pin':docs_pin},{'name':'documentation-selection.remote.json','pin':context['documentation']['selection']}]
    artifacts += [{'name':'publication-'+name+'.original','pin':value}for name,value in original_pins.items()]
    artifacts += [{'name':'documentation-'+name+'.original.json','pin':relocated[name]}for name in ['authentication','production']]
    retention={'source':source,'publicationIdentity':identity,'originalSelection':original_pin,'originalSixInputs':original_pins,'originalAcquisition':acquired,'originalDocumentationSelection':docs_pin,'originalDocumentationProofs':{k:relocated[k]for k in ['authentication','production']},'derivedPublicationSelection':context['publication']['selection'],'derivedVerifierPolicy':material.pin(remote_policy),'actualVerifier':verifier_pin,'derivedDocumentationSelection':context['documentation']['selection'],'publicTemplateManifest':manifest_pin,'artifactSubjects':artifacts,'scope':'Exact original data transport plus explicit physical path/UID0 locator projections. No source admission, cryptographic verification, capability serialization, guest approval, or runtime verdict. The135112256-byte verifier stays outside the128MiB-leaf approval inventory; only its fixed upstream identity/acquisition proof is copied into artifacts.'}
    save(root/'PUBLIC_INPUT_RETENTION.json',retention)
    return retention
