"""Fixed pre-start hijack, preserving every post-header byte without an HTTP read-ahead buffer."""
from engine import Connection
from plan import require
import re


def attach(deadline, cid):
    require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'UI exact attach owner ID')
    connection = Connection(deadline, 65534)
    connection.connect()
    stream = connection.sock
    try:
        path = '/v1.49/containers/' + cid + '/attach?logs=0&stream=1&stdin=1&stdout=1&stderr=1'
        stream.sendall(('POST ' + path + ' HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\n'
                        'Upgrade: tcp\r\nContent-Length: 0\r\n\r\n').encode('ascii'))
        header = bytearray()
        while not header.endswith(b'\r\n\r\n'):
            stream.guard()
            block = stream.recv(1)
            stream.guard()
            require(block and len(header) < 16384, 'UI bounded complete attach upgrade')
            header.extend(block)
        lines = bytes(header).decode('ascii', errors='strict').split('\r\n')
        require(lines[0].startswith('HTTP/1.1 101 '), 'UI actual duplex101 attach required')
        fields = {}
        for line in lines[1:-2]:
            key, separator, value = line.partition(':')
            require(separator and key.lower() not in fields, 'UI unique attach response header')
            fields[key.lower()] = value.strip().lower()
        require(fields.get('upgrade') == 'tcp' and fields.get('connection') == 'upgrade', 'UI actual TCP hijack')
        return stream
    except BaseException:
        stream.close()
        raise
