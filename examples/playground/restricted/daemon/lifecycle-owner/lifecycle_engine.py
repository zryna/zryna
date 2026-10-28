"""New private native-VM UID0 transport; the old UID65534 Engine remains unchanged."""
import base64
import json
import os
import socket
import struct
import time
from cli_common import require, sha256
from cli_engine import Connection, Chunked, decode
from cli_kernel_read import ticks


class Engine:
    def __init__(self, expected, deadline, record=None, peer_anchor=None):
        require(type(deadline) in [int, float] and time.monotonic() < deadline, 'finite selected phase deadline')
        self.expected, self.deadline, self.engine_uid = expected, deadline, 0
        self.record = record if record is not None else []
        self.peer_anchor = peer_anchor if peer_anchor is not None else []
        self.last_status = None

    def request(self, method, path, *, value=None, emitter=None, stream_maximum=0,
                statuses=(200, 201, 204), maximum_response=262144):
        require(method in ['GET', 'POST', 'PUT', 'DELETE'] and type(path) is str and path.startswith('/')
                and len(path) <= 4096 and '\r' not in path and '\n' not in path, 'bounded private Engine operation')
        require(type(maximum_response) is int and 0 < maximum_response <= 262144 and len(self.record) < 128, 'finite response/record count')
        require(value is None or emitter is None, 'one request body')
        connection, response = Connection(self.deadline, 0), None
        item = {'method': method, 'path': path, 'request': value, 'status': None, 'failure': None}
        self.record.append(item)
        try:
            connection.connect()  # Existing source checks socket ancestry/identity and SO_PEERCRED.
            pid, uid, gid = struct.unpack('3i', connection.sock.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            state = os.lstat('/var/run/docker.sock')
            peer = {'pid': pid, 'uid': uid, 'gid': gid, 'ticks': ticks(pid, self.deadline),
                    'socket': {'dev': state.st_dev, 'ino': state.st_ino, 'uid': state.st_uid, 'gid': state.st_gid,
                               'mode': state.st_mode, 'ctimeNs': str(state.st_ctime_ns)}}
            require(uid == 0, 'actual native Engine peer UID0')
            if self.peer_anchor:
                require(peer == self.peer_anchor[0], 'same actual Engine peer birth/socket')
            else:
                self.peer_anchor.append(peer)
            item['peer'] = peer
            emission = None
            if emitter is not None:
                require(type(stream_maximum) is int and 0 < stream_maximum <= 1075838976, 'existing finite USTAR transport bound')
                connection.putrequest(method, '/v1.49' + path)
                connection.putheader('Content-Type', 'application/x-tar')
                connection.putheader('Transfer-Encoding', 'chunked')
                connection.putheader('Connection', 'close')
                connection.endheaders()
                output = Chunked(connection, stream_maximum)
                emission = emitter(output)
                connection.send(b'0\r\n\r\n')
            else:
                body = b'' if value is None else json.dumps(value, separators=(',', ':')).encode()
                require(len(body) <= 262144, 'bounded request body')
                connection.request(method, '/v1.49' + path, body=body,
                                   headers={'Content-Type': 'application/json', 'Connection': 'close'})
            response = connection.getresponse()
            self.last_status = item['status'] = response.status
            raw = response.read(maximum_response + 1)
            item.update(rawBase64=base64.b64encode(raw).decode(), rawBytes=len(raw), rawSha256=sha256(raw))
            require(len(raw) <= maximum_response and response.length in [0, None] and response.read(1) == b''
                    and response.status in statuses and time.monotonic() < self.deadline,
                    'complete bounded original Engine response within phase')
            return raw, emission
        except BaseException as error:
            item['failure'] = {'type': type(error).__name__, 'errno': getattr(error, 'errno', None)}
            raise
        finally:
            try:
                if response is not None:
                    response.close()
            finally:
                connection.close()

    def json(self, method, path, value=None):
        return decode(self.request(method, path, value=value, statuses=(200, 201))[0])

    def audit(self):
        info, version = self.json('GET', '/info'), self.json('GET', '/version')
        require(info['ID'] == self.expected['id'] and info['ServerVersion'] == self.expected['version']
                and version['Version'] == self.expected['version'] and version['ApiVersion'] == self.expected['apiVersion']
                and info['CgroupDriver'] == self.expected['cgroupDriver'] and info['DockerRootDir'] == self.expected['dockerRootDir']
                and info['CgroupVersion'] == '2' and info['OSType'] == 'linux', 'same separately selected native Engine')
        for key in ['MemoryLimit', 'SwapLimit', 'PidsLimit', 'CpuCfsPeriod', 'CpuCfsQuota']:
            require(info[key] is True, 'declared kernel controller support; actual leaf/ancestors checked separately')
        return {'info': info, 'version': version}
