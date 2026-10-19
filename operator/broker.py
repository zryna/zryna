"""Trusted host transport only: real, unmodified duplex bytes to a private Engine.

This is not a guest service and grants no publication or runtime capability.
"""
import os
import select
import socket
import sys
import threading

backend = sys.argv[1]
assert os.geteuid() == 65534 and backend.startswith('/')
os.umask(0o007)
listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind('/var/run/docker.sock')
listener.listen(32)


def relay(client):
    upstream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        upstream.connect(backend)
        streams = [client, upstream]
        while streams:
            ready, _, _ = select.select(streams, [], [], 30)
            if not ready:
                return
            for source in ready:
                target = upstream if source is client else client
                data = source.recv(65536)
                if not data:
                    streams.remove(source)
                    target.shutdown(socket.SHUT_WR)
                else:
                    target.sendall(data)
    except (OSError, ValueError):
        pass
    finally:
        client.close()
        upstream.close()


while True:
    client, _ = listener.accept()
    threading.Thread(target=relay, args=(client,), daemon=True).start()
