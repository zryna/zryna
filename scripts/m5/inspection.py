"""Complete observed symbols/mnemonics and direct-call bytes; no reviewed policy."""
import re
import struct


def portion(data, start, length):
    assert 0 <= start <= len(data) and 0 <= length <= len(data) - start
    return data[start:start + length]


def string(data, offset):
    assert 0 <= offset < len(data)
    end = data.find(b'\0', offset)
    assert end >= offset
    return data[offset:end].decode('utf-8')


def observe(elf, disassembly, expected_undefined):
    assert 0 < len(elf) <= 32 * 1024 * 1024 and len(disassembly) <= 64 * 1024
    assert elf[:7] == b'\x7fELF\x02\x01\x01'
    header = struct.unpack('<16sHHIQQQIHHHHHH', portion(elf, 0, 64))
    assert header[1:4] == (2, 62, 1) and header[11] == 64
    section_start, section_count = header[6], header[12]
    assert section_count > 0
    section_bytes = portion(elf, section_start, section_count * 64)
    sections = [struct.unpack('<IIQQQQIIQQ', section_bytes[i:i + 64])
                for i in range(0, len(section_bytes), 64)]
    for section in sections:
        if section[1] != 8:  # NOBITS has no file payload.
            portion(elf, section[4], section[5])
    undefined, functions = [], {}
    for section in sections:
        if section[1] not in (2, 11):
            continue
        assert section[9] == 24 and section[5] % 24 == 0
        assert section[6] < len(sections)
        strings = sections[section[6]]
        assert strings[1] == 3
        names = portion(elf, strings[4], strings[5])
        symbols = portion(elf, section[4], section[5])
        for offset in range(0, len(symbols), 24):
            name_offset, info, _, index, address, size = struct.unpack(
                '<IBBHQQ', symbols[offset:offset + 24])
            name = string(names, name_offset)
            if section[1] == 11 and index == 0 and name:
                undefined.append(name)
            if name in ('__wrap_sum_bytes', 'sum_bytes') and info & 15 == 2 and index != 0:
                assert index < len(sections) and size > 0
                code_section = sections[index]
                assert code_section[2] & 4 and address >= code_section[3]
                displacement = address - code_section[3]
                assert displacement + size <= code_section[5]
                code = portion(elf, code_section[4] + displacement, size)
                value = (address, code)
                assert name not in functions or functions[name] == value
                functions[name] = value
    undefined.sort()
    assert undefined == expected_undefined and undefined
    assert set(functions) == {'__wrap_sum_bytes', 'sum_bytes'}
    start, code = functions['__wrap_sum_bytes']
    end = start + len(code)
    mnemonics, instructions, calls, seen = [], [], [], set()
    for line in disassembly.decode('utf-8').splitlines():
        fields = line.split(':', 1)
        if len(fields) != 2 or re.fullmatch(r'\s*[0-9a-f]+\s*', fields[0]) is None:
            continue  # Section headings and the symbol heading have no instruction address.
        words = fields[1].strip().split(None, 1)
        assert words, 'instruction address has no mnemonic'
        site, mnemonic = int(fields[0].strip(), 16), words[0]
        operands = words[1] if len(words) > 1 else ''
        assert start <= site < end and site not in seen
        assert not instructions or site > instructions[-1]['site']
        seen.add(site)
        mnemonics.append(mnemonic)
        instructions.append({'site': site, 'mnemonic': mnemonic, 'operands': operands})
        if mnemonic in ('call', 'callq'):
            target = re.fullmatch(r'([0-9a-f]+) <([^<>]+)>', operands)
            assert target is not None
            position = site - start
            raw = portion(code, position, 5)
            assert raw[0] == 0xe8
            called = site + 5 + struct.unpack('<i', raw[1:])[0]
            assert called == int(target[1], 16)
            calls.append({'site': site, 'target': called, 'name': target[2],
                          'directCallBytes': raw.hex()})
    assert instructions and instructions[0]['site'] == start
    forward = [x for x in calls if x['name'] == 'sum_bytes']
    assert len(forward) == 1 and forward[0]['target'] == functions['sum_bytes'][0]
    return {'undefinedDynamicSymbols': undefined, 'orderedMnemonics': mnemonics,
            'instructions': instructions, 'calls': calls,
            'observedOnly': True, 'reviewedBindings': False,
            'targetsChecked': False, 'targetExecution': 'UNRUN'}
