"""Audit source-owned include/path dependencies across all cfg branches, without Git claims.

This deliberately small lexical reader never evaluates Rust. Unsupported include expressions
or include-bearing macro forms fail closed; the ordinary compiler remains the authority.
"""
from pathlib import PurePosixPath
import json
import posixpath
import re

INCLUDES = {'include', 'include_str', 'include_bytes'}


def tokens(source):
    result, index = [], 0
    while index < len(source):
        tail = source[index:]
        if tail[0].isspace():
            index += 1
            continue
        if tail.startswith('//'):
            end = source.find('\n', index)
            index = len(source) if end < 0 else end + 1
            continue
        if tail.startswith('/*'):
            depth, index = 1, index + 2
            while depth:
                assert index < len(source), 'unterminated Rust comment'
                if source.startswith('/*', index):
                    depth, index = depth + 1, index + 2
                elif source.startswith('*/', index):
                    depth, index = depth - 1, index + 2
                else:
                    index += 1
            continue
        raw = re.match(r'(?:br|cr|r)(#*)"', tail)
        if raw:
            endmark = '"' + raw[1]
            end = source.find(endmark, index + raw.end())
            assert end >= 0, 'unterminated raw Rust string'
            end += len(endmark)
            result.append(source[index:end])
            index = end
            continue
        quoted = re.match(r'(?:b|c)?"', tail)
        if quoted:
            end = index + quoted.end()
            while end < len(source):
                if source[end] == '\\':
                    end += 2
                elif source[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
            else:
                raise AssertionError('unterminated Rust string')
            result.append(source[index:end])
            index = end
            continue
        character = re.match(r"(?:b)?'(?:\\u\{[0-9a-fA-F_]+\}|\\x[0-9a-fA-F]{2}|\\.|[^'\\\n])'", tail)
        if character:
            result.append(character[0])
            index += character.end()
            continue
        word = re.match(r'[A-Za-z_][A-Za-z_0-9]*', tail)
        value = word[0] if word else tail[0]
        result.append(value)
        index += len(value)
    return result


def group(values, start):
    pairs = {'(': ')', '[': ']', '{': '}'}
    assert start < len(values) and values[start] in pairs, 'expected Rust token group'
    closing, index = pairs[values[start]], start + 1
    while index < len(values):
        if values[index] == closing:
            return values[start + 1:index], index + 1
        if values[index] in pairs:
            _, index = group(values, index)
        else:
            assert values[index] not in pairs.values(), 'mismatched Rust token group'
            index += 1
    raise AssertionError('unclosed Rust token group')


def literal(value):
    if value.startswith('"'):
        return json.loads(value)
    raw = re.fullmatch(r'r(#+)?"(.*)"\1', value, re.DOTALL)
    if raw:
        return raw[2]
    if value.startswith('r"') and value.endswith('"'):
        return value[2:-1]
    raise AssertionError(('unsupported Rust path literal', value))


def expression(values, variables):
    if len(values) == 1:
        return literal(values[0])
    if len(values) == 2 and values[0] == '$':
        assert values[1] in variables, 'unbound include macro parameter'
        return variables[values[1]]
    assert len(values) >= 4 and values[1:3] == ['!', '('], ('unsupported include expression', values)
    arguments, end = group(values, 2)
    assert end == len(values)
    if values[0] == 'env':
        assert arguments == ['"CARGO_MANIFEST_DIR"'], 'unsupported source include environment'
        return '@MANIFEST@'
    assert values[0] == 'concat', ('unsupported source include macro', values[0])
    parts, start, index = [], 0, 0
    while index < len(arguments):
        if arguments[index] in ('(', '[', '{'):
            _, index = group(arguments, index)
        elif arguments[index] == ',':
            parts.append(expression(arguments[start:index], variables))
            start = index = index + 1
        else:
            index += 1
    if start < len(arguments):
        parts.append(expression(arguments[start:], variables))
    assert parts, 'empty include concatenation'
    return ''.join(parts)


def references(source):
    values = tokens(source)
    result, macros, index = [], {}, 0
    while index < len(values):
        if values[index:index + 2] == ['macro_rules', '!']:
            name = values[index + 2]
            body, end = group(values, index + 3)
            if any(item in INCLUDES for item in body) or any(
                    body[position:position + 2] == ['path', '='] for position in range(len(body))):
                formal, position = group(body, 0)
                assert body[position:position + 2] == ['=', '>'], 'unsupported include-bearing macro rule'
                replacement, position = group(body, position + 2)
                assert body[position:] in ([], [';']), 'multiple include-bearing macro rules'
                names, offset = [], 0
                while offset < len(formal):
                    assert formal[offset] == '$' and formal[offset + 2:offset + 4] == [':', 'literal'], 'include macro requires literal arguments'
                    names.append(formal[offset + 1])
                    offset += 4
                    if offset < len(formal):
                        assert formal[offset] == ','
                        offset += 1
                assert name not in macros and names
                macros[name] = (names, replacement)
            values[index:end] = []
        else:
            index += 1

    def scan(items, variables):
        index = 0
        while index < len(items):
            item = items[index]
            if item in INCLUDES and items[index + 1:index + 2] == ['!']:
                arguments, end = group(items, index + 2)
                if arguments[-1:] == [',']:
                    arguments = arguments[:-1]
                result.append(expression(arguments, variables))
                index = end
            elif items[index:index + 2] == ['#', '[']:
                attribute, end = group(items, index + 1)
                for offset, token in enumerate(attribute):
                    if token == 'path':
                        assert attribute[offset + 1:offset + 2] == ['='], 'unsupported path attribute'
                        result.append(literal(attribute[offset + 2]))
                        assert attribute[offset + 3:offset + 4] in ([], [','], [')']), 'unsupported path attribute value'
                scan(attribute, variables)
                index = end
            elif item in macros and items[index + 1:index + 2] == ['!']:
                arguments, end = group(items, index + 2)
                names, replacement = macros[item]
                actual = arguments[::2]
                assert all(v == ',' for v in arguments[1::2]) and len(actual) == len(names)
                scan(replacement, dict(zip(names, map(literal, actual))))
                index = end
            else:
                index += 1

    scan(values, {})
    return result


def audit(source_rows, source_bytes, selected):
    """Use only authenticated rows/bytes supplied by the owning mirror, never reports."""
    available = {row['path'] for row in source_rows}
    pending = sorted(path for path in selected if path.endswith('.rs'))
    examined, edges = set(), []
    while pending:
        origin = pending.pop()
        if origin in examined:
            continue
        examined.add(origin)
        for reference in references(source_bytes(origin).decode('utf-8')):
            if reference.startswith('@MANIFEST@/'):
                path = 'crates/zryna-driver/' + reference.removeprefix('@MANIFEST@/')
            else:
                assert not reference.startswith('/') and '@MANIFEST@' not in reference
                path = str(PurePosixPath(origin).parent / reference)
            path = posixpath.normpath(path)
            assert not path.startswith('../') and path not in ('.', '..')
            assert path in available, ('include/path missing from complete source census', origin, path)
            assert path in selected, ('include/path missing from owned mirror closure', origin, path)
            edges.append({'from': origin, 'to': path})
            if path.endswith('.rs'):
                pending.append(path)
    return {'rustFilesExamined': sorted(examined), 'includePathEdges': sorted(edges, key=lambda edge: (edge['from'], edge['to'])),
            'cfgPolicy': 'all source branches, including inactive and cross-platform branches'}
