"""Interpret one exact originating link trace; never dereference trace tokens."""
import re

LOCAL = re.compile(r'/proc/[1-9][0-9]*/fd/[1-9][0-9]*/(?:runtime|program)\.o')
ROOTS = ('/usr/lib/', '/usr/lib64/', '/lib/', '/lib64/')


def argument(data):
    assert isinstance(data, list) and all(type(x) is int and 0 <= x <= 255 for x in data)
    return bytes(data).decode('utf-8')


def select_trace(raw, record):
    assert record['name'] == 'link' and record['success'] is True
    assert record['diagnostic'] is None and len(raw) <= 64 * 1024
    args = [argument(x) for x in record['arguments_bytes']]
    local = [x for x in args if LOCAL.fullmatch(x)]
    assert len(local) == len(set(local)) == 4
    assert sum(x.endswith('/program.o') for x in local) == 1
    assert '-Wl,-t,-t' in args and '-fsanitize=address,undefined' in args
    observed_local, providers, occurrences, folded = set(), [], [], {}
    for token in raw.decode('utf-8').splitlines():
        if not token:
            continue
        assert token == token.strip() and len(token) <= 4096 and '\0' not in token
        if token in local:
            assert token not in observed_local
            observed_local.add(token)
            continue
        assert not token.startswith('/proc/') and token.startswith(ROOTS)
        assert not any(c.isspace() for c in token) and not any(c in token for c in '()\\')
        assert token.casefold() not in folded or folded[token.casefold()] == token
        folded[token.casefold()] = token
        occurrences.append(token)
        if token not in providers:
            providers.append(token)
            assert len(providers) <= 64
    assert observed_local == set(local) and providers
    return {'paths': providers, 'occurrences': occurrences, 'localInputs': local}
