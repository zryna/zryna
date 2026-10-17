"""Closed scalar helpers for the operator-only CLI owner."""
import hashlib
import re


def require(value, message):
    if not value:
        raise ValueError('CLI-OWNER: ' + message)


def exact(value, keys):
    require(type(value) is dict and set(value) == set(keys), 'closed object')


def integer(value, maximum, minimum=0):
    require(type(value) is int and minimum <= value <= maximum, 'bounded integer')


def digest(value):
    require(type(value) is str and re.fullmatch('[a-f0-9]{64}', value), 'SHA256')


def sha256(value):
    return hashlib.sha256(value).hexdigest()


ENV = ['PATH=/opt/zryna-release/runtime/node/bin', 'LC_ALL=C.UTF-8', 'LANG=C.UTF-8',
       'TMPDIR=/tmp', 'HOME=/nonexistent', 'TZ=UTC', 'SOURCE_DATE_EPOCH=0']
