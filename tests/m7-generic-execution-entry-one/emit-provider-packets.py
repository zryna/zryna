#!/usr/bin/env python3
"""Real frozen two-module syntax processes; no source/compiler/backend authority claimed."""
from pathlib import Path
import hashlib,importlib.util,json
BASE=Path(__file__).resolve().parent
MECHANISM=Path('/tmp/m416-nested-nominal-proposal/fixtures/emit-provider-packets.py')
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def dump(path,value): path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def check_inputs():
    frozen=json.loads((BASE/'INPUT_FREEZE.json').read_bytes())
    for row in frozen['inputs']:
        path=Path(row['path']);assert path.stat().st_size==row['bytes'] and digest(path)==row['sha256'],str(path)
    return frozen
inputs=check_inputs()
spec=importlib.util.spec_from_file_location('pinned_mechanism',MECHANISM)
mechanism=importlib.util.module_from_spec(spec);spec.loader.exec_module(mechanism)
mechanism.BASE=BASE
assert str(mechanism.NATIVE)==inputs['native']['path']
assert digest(mechanism.NATIVE)==inputs['native']['sha256']
assert digest(mechanism.NODE)==inputs['typescript']['node_sha256']
assert mechanism.ts_material()==inputs['typescript']['material']
freeze=json.loads((BASE/'SOURCE_FREEZE.json').read_bytes())
for row in freeze['files']:assert digest(BASE/row['path'])==row['sha256']
expected=json.loads((BASE/'EXPECTED.json').read_bytes())
files=[{'path':row['path'],'text':(BASE/row['literal']).read_text()} for row in expected['sources']]
records=[];packets=[]
for provider in ['native','typescript']:
    record,packet=mechanism.emit(expected['case'],provider,files,digest(BASE/'SOURCE_FREEZE.json'))
    record['acceptedSyntaxWithoutDiagnostics']=True
    record['exactSources']=[{'path':row['path'],'sha256':row['sha256']} for row in expected['sources']]
    records.append(record);packets.append(packet)
    print(provider,'actual process accepted two complete files',flush=True)
assert packets[0]==packets[1],'complete DTO parity'
check_inputs()
assert mechanism.ts_material()==inputs['typescript']['material']
dump(BASE/'provider-receipts.json',{'status':'Two actual syntax-only provider processes; no authenticated source, semantic, issuer or target execution claim','sourceExpectedFreezeSha256':digest(BASE/'SOURCE_FREEZE.json'),'records':records})
dump(BASE/'provider-outcomes.json',{'status':'Actual syntax-only observations; frozen sources/oracle unchanged','freshProcesses':2,'acceptedWithoutDiagnostics':2,'completeDTOParity':True,'sourcePaths':[row['path'] for row in files],'declaredEntryModule':1})
dump(BASE/'packet-identities.json',{'status':'Complete packets from named actual provider processes','originalReceiptsSha256':digest(BASE/'provider-receipts.json'),'records':[{key:row[key] for key in ['case','provider','packet','packetBytes','packetSha256','exactSources','acceptedSyntaxWithoutDiagnostics']} for row in records]})
print('2 real processes; complete DTO parity',flush=True)
