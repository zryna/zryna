"""Inert fixtures: no Docker socket, installed CLI, provider, browser or generated code."""
import copy
import io
import json
from pathlib import Path
import struct
import tarfile
import time
from cli_common import ENV, sha256
from cli_config import SECCOMP_SHA256
from cli_plan import PLAN_SHA256

CID, EID = 'b' * 64, 'e' * 64


def approval():
    policy = Path(__file__).with_name('compiler-seccomp.json').read_text()
    return {'format': 'zryna.playground-cli-owner-approval.v1', 'version': 1,
            'image': 'sha256:' + 'a' * 64, 'layer_sha256': 'c' * 64, 'image_plan_sha256': 'd' * 64,
            'seccomp': policy, 'seccomp_sha256': SECCOMP_SHA256, 'command_plan_sha256': PLAN_SHA256}


def project_tar(operation, populated):
    import base64
    entries = [('project', None, 0o700)]
    if populated:
        entries.append(('project/src', None, 0o700))
        entries += [('project/' + item['path'], base64.b64decode(item['bodyBase64']), 0o600)
                    for item in operation['projectFiles']]
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w', format=tarfile.USTAR_FORMAT) as archive:
        for name, data, mode in entries:
            info = tarfile.TarInfo(name)
            info.uid, info.gid, info.mode = 65534, 65534, mode
            if data is None:
                info.type = tarfile.DIRTYPE
                archive.addfile(info)
            else:
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
    return output.getvalue()


class Engine:
    def __init__(self, operation):
        self.operation = operation
        self.engine_uid, self.deadline = 65534, time.monotonic() + 9
        self.state, self.calls, self.populated = None, [], False
        self.deleted = self.exec_started = self.lose_create = self.cleanup_fail = False
        self.last_status = 200

    def json(self, method, path, value=None):
        self.calls.append((method, path))
        if path.startswith('/images/'):
            return {'Id': approval()['image'], 'Os': 'linux', 'Architecture': 'amd64',
                    'RootFS': {'Type': 'layers', 'Layers': ['sha256:' + approval()['layer_sha256']]},
                    'Config': {'User': '65534:65534', 'Entrypoint': ['/supervisor'],
                               'Cmd': ['idle'], 'WorkingDir': '/work/project', 'Env': ENV}}
        if path.startswith('/containers/create?'):
            self.state = {'Id': CID, 'Name': '/' + path.split('name=')[1], 'Image': value['Image'],
                'Config': {key: copy.deepcopy(item) for key, item in value.items() if key != 'HostConfig'},
                'HostConfig': copy.deepcopy(value['HostConfig']), 'RestartCount': 0,
                'Mounts': [{'Type': 'tmpfs', 'Destination': dest} for dest in value['HostConfig']['Tmpfs']],
                'State': {'Status': 'created', 'Pid': 0, 'Running': False, 'Paused': False, 'Restarting': False,
                          'OOMKilled': False, 'Error': '', 'ExitCode': 0,
                          'StartedAt': '0001-01-01T00:00:00Z', 'FinishedAt': '0001-01-01T00:00:00Z'}}
            if self.lose_create:
                raise TimeoutError('inert lost create response')
            return {'Id': CID}
        if path.endswith('/exec'):
            self.exec_request = value
            return {'Id': EID}
        if path.startswith('/exec/'):
            return {'ID': EID, 'ContainerID': CID, 'Running': False, 'ExitCode': 0,
                    'OpenStdin': False, 'OpenStdout': True, 'OpenStderr': True, 'ProcessConfig': {
                        'entrypoint': self.exec_request['Cmd'][0], 'arguments': self.exec_request['Cmd'][1:],
                        'privileged': False, 'tty': False, 'user': '65534:65534', 'working_dir': '/work/project'}}
        return copy.deepcopy(self.state)

    def request(self, method, path, **kwargs):
        self.calls.append((method, path))
        if self.cleanup_fail and ('/kill?' in path or method == 'DELETE'):
            raise TimeoutError('inert cleanup failure')
        self.last_status = 200
        if path.endswith('/start') and not path.startswith('/exec/'):
            self.state['State'].update(Status='running', Pid=321, Running=True, StartedAt='2026-10-16T00:00:00Z')
        elif '/archive?' in path and method == 'GET':
            return project_tar(self.operation, self.populated), None
        elif '/archive?' in path and method == 'PUT':
            self.populated = True
            class Output:
                def write(self, data):
                    return len(data)
            return b'', kwargs['emitter'](Output())
        elif path.startswith('/exec/') and path.endswith('/start'):
            self.exec_started = True
            raw = b'{"synthetic":"never actual CLI evidence"}'
            return struct.pack('>BBBBI', 1, 0, 0, 0, len(raw)) + raw, None
        elif '/kill?' in path:
            self.state['State'].update(Status='exited', Pid=0, Running=False, ExitCode=137)
        elif '/wait?' in path:
            return json.dumps({'StatusCode': self.state['State']['ExitCode']}).encode(), None
        elif method == 'DELETE':
            self.deleted, self.last_status = True, 204
        elif method == 'GET' and path.endswith('/json'):
            self.last_status = 404 if self.deleted else 200
            return json.dumps({'message': 'No such container'} if self.deleted else self.state).encode(), None
        return b'', None
