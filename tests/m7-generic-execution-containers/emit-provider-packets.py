#!/usr/bin/env python3
"""Transparent syntax-only observation of immutable source/oracle inputs. Root executes."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

BASE = Path(__file__).resolve().parent
ROOT = Path('/tmp/m416-nominal-transport-candidate')
ADAPTER = ROOT / 'adapters/typescript-6'
NATIVE = Path('/tmp/m416-target/debug/deps/worker_process-a0ba6f0a0b2c8980')
NODE = Path('/workspace/.cloud-setup/node-v22.22.1-linux-x64/bin/node')
EXPECTED_NATIVE = '27d5ff7f8f012d2addaaeaa61c3a14aac5062d627f015b7ed73166e8cba7ea45'
EXPECTED_SOURCE = 'd7d6d51db3eb9b51ed729b32aac8590a05b3102056ee2f88bc9c8baa51530d33'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def dump(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')

def verify():
    pins = json.loads((BASE / 'INPUT_FREEZE.json').read_bytes())
    for row in pins['inputs']:
        path = Path(row['path'])
        assert path.stat().st_size == row['bytes'] and digest(path) == row['sha256'], str(path)
    assert digest(NATIVE) == EXPECTED_NATIVE
    assert digest(BASE / 'SOURCE_EXPECTED_FREEZE.json') == EXPECTED_SOURCE
    rows = json.loads((BASE / 'SOURCE_EXPECTED_FREEZE.json').read_bytes())['cases']
    for row in rows:
        literal = BASE / 'literals' / (row['case'] + '.zry')
        assert digest(literal) == row['sha256'] == digest(Path(row['source']))
    return rows

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--execute', action='store_true')
    parser.add_argument('--output', type=Path, default=BASE / 'actual')
    args = parser.parse_args()
    assert args.execute, 'Only root executes providers after reviewing immutable source expectations'
    rows = verify()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    assert not (output / 'provider-receipts.json').exists(), 'Use a fresh output directory; do not overwrite observations'
    records, outcomes = [], []
    for row in rows:
        pair, accepted = [], []
        for provider in ['native', 'typescript']:
            verify()
            command = [str(NATIVE), '--native-worker-v5'] if provider == 'native' else [str(NODE), 'src/worker-v5.mjs']
            cwd = ROOT if provider == 'native' else ADAPTER
            requests = [{'id': 1, 'method': 'handshake'}, {'id': 2, 'method': 'analyze',
                         'params': {'schema_version': 5, 'files': [{'path': 'main.zry', 'text': (BASE / 'literals' / (row['case'] + '.zry')).read_text()}]}}]
            wire = ''.join(json.dumps(request, ensure_ascii=False, separators=(',', ':')) + '\n' for request in requests).encode()
            prefix = output / (row['case'] + '-' + provider)
            Path(str(prefix) + '.stdin.jsonl').write_bytes(wire)
            process, failure = None, None
            try:
                process = subprocess.run(command, cwd=cwd, input=wire, stdout=subprocess.PIPE,
                                         stderr=subprocess.PIPE, check=False, timeout=30)
                stdout, stderr = process.stdout, process.stderr
            except subprocess.TimeoutExpired as error:
                stdout, stderr = error.stdout or b'', error.stderr or b''
                failure = {'type': 'TimeoutExpired', 'seconds': 30}
            Path(str(prefix) + '.stdout.jsonl').write_bytes(stdout)
            Path(str(prefix) + '.stderr.txt').write_bytes(stderr)
            responses, parse_failures = [], []
            for index, line in enumerate(stdout.splitlines()):
                try:
                    responses.append(json.loads(line))
                except Exception as error:
                    parse_failures.append({'line': index + 1, 'error': str(error)})
            handshake = next((response for response in responses if response.get('id') == 1), {})
            analysis = next((response for response in responses if response.get('id') == 2), {})
            packet = analysis.get('result')
            named = None
            if packet is not None:
                named = row['case'] + '-' + provider + '.json'
                dump(output / named, packet)
            okay = bool(process and process.returncode == 0 and not stderr and not parse_failures
                        and len(responses) == 2 and handshake.get('result', {}).get('protocol_version') == 5
                        and handshake.get('result', {}).get('provider') == ('zryna-native' if provider == 'native' else 'typescript-6')
                        and packet is not None and packet.get('schema_version') == 5 and packet.get('diagnostics') == []
                        and 'error' not in analysis)
            verify()
            record = {'case': row['case'], 'provider': provider, 'command': command, 'cwd': str(cwd),
                      'sourceExpectedFreezeSha256': EXPECTED_SOURCE, 'exactSourceSha256': row['sha256'],
                      'inputPinFreezeSha256': digest(BASE / 'INPUT_FREEZE.json'),
                      'handshake': handshake.get('result'), 'returncode': process.returncode if process else None,
                      'failure': failure, 'jsonLineErrors': parse_failures,
                      'analysisError': analysis.get('error'), 'diagnostics': packet.get('diagnostics') if packet else None,
                      'stdinSha256': digest(Path(str(prefix) + '.stdin.jsonl')),
                      'stdoutSha256': digest(Path(str(prefix) + '.stdout.jsonl')), 'stderrBytes': len(stderr),
                      'packet': named, 'packetSha256': digest(output / named) if named else None,
                      'packetBytes': (output / named).stat().st_size if named else None,
                      'acceptedSyntaxWithoutDiagnostics': okay}
            records.append(record); pair.append(packet); accepted.append(okay)
        parity = pair[0] is not None and pair[0] == pair[1]
        outcomes.append({'case': row['case'], 'nativeAccepted': accepted[0], 'typescriptAccepted': accepted[1],
                         'completeDTOEqual': parity, 'completeAcceptedDTOParity': parity and all(accepted)})
        print(row['case'], accepted, 'completeAcceptedDTOParity', outcomes[-1]['completeAcceptedDTOParity'], flush=True)
    verify()
    dump(output / 'provider-receipts.json', {'status': 'Actual fresh syntax-only processes; complete local stdin/stdout/stderr retained, every failure preserved. No compiler source/semantic/seal/runtime claim.', 'records': records})
    dump(output / 'provider-outcomes.json', {'freshProviderProcesses': len(records), 'cases': outcomes})
    dump(output / 'packet-identities.json', {'status': 'Named packets are actual process outputs, not source/semantic/ownership authority.', 'records': [{key: record[key] for key in ['case', 'provider', 'packet', 'packetSha256', 'packetBytes', 'exactSourceSha256', 'acceptedSyntaxWithoutDiagnostics']} for record in records]})
    schema = subprocess.run([str(NODE), str(BASE / 'validate-packets.mjs'), str(output)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    (output / 'schema.stdout.txt').write_bytes(schema.stdout); (output / 'schema.stderr.txt').write_bytes(schema.stderr)
    print(schema.stdout.decode(), end='', flush=True)
    if schema.stderr:
        print(schema.stderr.decode(), end='', flush=True)
    verify()
    assert schema.returncode == 0, 'Actual strict schema validation failed; observations preserved'
    assert all(outcome['completeAcceptedDTOParity'] for outcome in outcomes), 'Provider rejection/disagreement preserved; do not change source/oracles to fit it'
    print('18 fresh provider processes, 9 complete accepted DTO parity cases, strict schema qualified; later compiler phases unqualified', flush=True)

if __name__ == '__main__':
    main()
