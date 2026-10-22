"""Fixed UI proc-subpath profile plus exact observed null-mask mount topology."""
import re

PROC_READONLY = ('/proc/bus', '/proc/fs', '/proc/irq', '/proc/sys', '/proc/sysrq-trigger')


def require(value, reason):
    if not value:
        raise ValueError('UI-MOUNTS: ' + reason)


def topology(text, cid):
    require(type(cid) is str and re.fullmatch('[a-f0-9]{64}', cid), 'exact original container ID')
    by_id, groups = {}, {}
    for line in text.splitlines():
        left, separator, right = line.partition(' - ')
        before, after = left.split(), right.split()
        require(separator and len(before) >= 6 and len(after) == 3 and len(by_id) < 512,
                'complete bounded mountinfo rows')
        require(re.fullmatch('[1-9][0-9]{0,9}', before[0]) and
                re.fullmatch('0|[1-9][0-9]{0,9}', before[1]), 'numeric mount identities')
        mount_id, parent = int(before[0]), int(before[1])
        target = before[4]
        require(mount_id <= 4294967295 and parent <= 4294967295 and mount_id not in by_id and
                mount_id != parent and target.startswith('/') and '\\' not in target and
                (target == '/' or not target.endswith('/') and '//' not in target) and
                all(part not in ['.', '..'] for part in target.split('/')), 'unique exact mount identity/path')
        require(re.fullmatch('[0-9]+:[0-9]+', before[2]), 'actual filesystem device identity')
        row = {'id': mount_id, 'parentId': parent, 'device': before[2], 'root': before[3],
               'options': before[5].split(','), 'filesystem': after[0], 'source': after[1],
               'superOptions': after[2].split(',')}
        by_id[mount_id] = row
        groups.setdefault(target, []).append(row)
    require('/' in groups and len(groups['/']) == 1, 'sole runtime root mount')
    result = {}
    for target, rows in groups.items():
        if len(rows) == 1:
            result[target] = rows[0]
            continue
        require(target in ['/proc', '/etc/hostname', '/etc/hosts', '/etc/resolv.conf'] and len(rows) == 2,
                'only exact observed two-layer proc/null stacks')
        children = [(base, top) for base in rows for top in rows if top['parentId'] == base['id']]
        require(len(children) == 1, 'unique acyclic same-target parent-child stack')
        base, top = children[0]
        require(base['parentId'] not in [row['id'] for row in rows], 'connected base outside stack')
        if target == '/proc':
            require(base['root'] == top['root'] == '/' and base['device'] == top['device'] and
                    base['filesystem'] == top['filesystem'] == 'proc' and base['source'] == top['source'] == 'proc' and
                    {'rw', 'nosuid', 'nodev', 'noexec'} <= set(base['options']) and 'ro' not in base['options'] and
                    {'ro', 'nosuid', 'nodev', 'noexec'} <= set(top['options']) and 'rw' not in top['options'] and
                    base['superOptions'] == top['superOptions'], 'exact read-only remount of original proc')
        else:
            require('/dev' in groups and len(groups['/dev']) == 1, 'sole fixed dev tmpfs backing null masks')
            dev = groups['/dev'][0]
            require(base['root'] == '/var/lib/docker/containers/' + cid + '/' + target.rsplit('/', 1)[1] and
                    base['filesystem'] == 'overlay' and base['source'] == 'overlay' and
                    'ro' in base['options'] and 'rw' not in base['options'] and
                    dev['root'] == '/' and dev['filesystem'] == dev['source'] == 'tmpfs' and
                    {'rw', 'nosuid', 'noexec'} <= set(dev['options']) and
                    top['root'] == '/null' and top['filesystem'] == top['source'] == 'tmpfs' and
                    top['device'] == dev['device'] and top['superOptions'] == dev['superOptions'] and
                    {'rw', 'nosuid', 'noexec'} <= set(top['options']) and 'ro' not in top['options'],
                    'exact Docker file masked by same-dev null tmpfs bind')
        result[target] = {**top, 'stack': {'base': base, 'effectiveId': top['id']}}
    root_id = result['/']['id']
    for target, rows in groups.items():
        for row in rows:
            if row['id'] == root_id:
                require(row['parentId'] not in by_id, 'namespace root external parent')
                continue
            require(row['parentId'] in by_id, 'complete connected mount tree')
            parent_target = next(name for name, entries in groups.items() if by_id[row['parentId']] in entries)
            require(target == parent_target or target.startswith(parent_target.rstrip('/') + '/'),
                    'mount parent is actual path ancestor')
            if target != parent_target:
                require(row['parentId'] == result[parent_target]['id'], 'no child hidden under covered stack base')
            seen, cursor = set(), row['id']
            while cursor != root_id:
                require(cursor in by_id and cursor not in seen, 'acyclic complete mount ancestry')
                seen.add(cursor)
                cursor = by_id[cursor]['parentId']
    for target in ['/', '/materials', '/sys/fs/cgroup']:
        require(target in result and 'ro' in result[target]['options'] and 'rw' not in result[target]['options'],
                'actual read-only mount ' + target)
    require(result['/sys/fs/cgroup']['filesystem'] == 'cgroup2', 'actual read-only cgroup2 mount')
    for target in ['/etc/hostname', '/etc/hosts', '/etc/resolv.conf']:
        if target in result:
            require('stack' in result[target] and result[target]['root'] == '/null',
                    'present Docker host file is actually null-masked')
    require(not any(target.startswith('/materials/') for target in result), 'no material subtree overmount')
    require(not any(target.startswith('/tmp/') or target.startswith('/dev/shm/') for target in result),
            'no bounded scratch subtree overmount')
    for target in result:
        require(not any(target == '/' + name or target.startswith('/' + name + '/')
                        for name in ['app', 'resources', 'lib', 'lib64', 'supervisor', 'ui-supervisor', 'etc/fonts']),
                'no approved runtime image overmount')
    require('/etc' not in result, 'no fixed fontconfig ancestor overmount')
    return result


def resolve(text, cid, proc_proof=None):
    from owner_audit import proc_description
    proc_receipt = proc_description(proc_proof, cid)['receipt'] if proc_proof is not None else None
    result = topology(text, cid)
    require('/proc' in result, 'actual fixed UI proc mount exists')
    proc = result['/proc']
    require('stack' not in proc and proc['root'] == '/' and proc['filesystem'] == proc['source'] == 'proc' and
            {'rw', 'nosuid', 'nodev', 'noexec'} <= set(proc['options']) and 'ro' not in proc['options'],
            'sole UI proc permits own sandbox namespace maps with fixed safety flags')
    for target in PROC_READONLY:
        require(not any(path.startswith(target + '/') for path in result), 'no protected proc subtree overmount')
        if target == '/proc/sysrq-trigger' and target not in result:
            require(proc_receipt is not None and proc_receipt['sysrqTrigger'] == 'absent-enoent',
                    'missing sysrq interface requires actual same-container native ENOENT proof')
            continue
        if target == '/proc/sysrq-trigger' and proc_receipt is not None:
            require(proc_receipt['sysrqTrigger'] == 'present-readonly', 'actual proc proof matches present interface')
        require(target in result, 'all five fixed proc protections exist')
        cell = result[target]
        require('stack' not in cell and cell['parentId'] == proc['id'] and
                cell['root'] == target[len('/proc'):] and cell['device'] == proc['device'] and
                cell['filesystem'] == cell['source'] == 'proc' and cell['superOptions'] == proc['superOptions'] and
                {'ro', 'nosuid', 'nodev', 'noexec'} <= set(cell['options']) and 'rw' not in cell['options'],
                'exact read-only bind of selected proc protection')
    return result
