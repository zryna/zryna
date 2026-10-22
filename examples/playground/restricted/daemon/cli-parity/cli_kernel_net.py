"""Actual namespace and no-external-route observations for the fixed CLI role."""
import os
import re
import time
from cli_kernel_read import read, require

def namespace_links(pid, deadline):
    result = {}
    for subject in ['self', str(pid)]:
        for kind in ['user', 'pid', 'mnt', 'net', 'cgroup']:
            require(time.monotonic() < deadline, 'namespace observation deadline')
            try:
                link = os.readlink(f'/proc/{subject}/ns/{kind}')
                require(re.fullmatch(kind + r':\[[0-9]+\]', link), 'actual namespace link syntax')
                result[subject + '/' + kind] = {'actualLink': link}
            except PermissionError as error:
                result[subject + '/' + kind] = {'actualLink': None, 'unavailableErrno': error.errno}
    for kind in ['pid', 'mnt', 'net', 'cgroup']:
        observer = result['self/' + kind]['actualLink']
        target = result[str(pid) + '/' + kind]['actualLink']
        require(observer is None or target is None or observer != target, 'observable private namespace ' + kind)
    return result


def network(pid, deadline, links):
    devices = read(f'/proc/{pid}/net/dev', deadline)
    names = []
    for line in devices.splitlines()[2:]:
        name, colon, counters = line.partition(':')
        require(colon and len(counters.split()) == 16 and all(x.isdigit() for x in counters.split()),
                'actual network interface counters')
        names.append(name.strip())
    require(names == ['lo'], 'only private loopback interface')
    route = read(f'/proc/{pid}/net/route', deadline)
    require(len(route.splitlines()) == 1 and route.split()[0] == 'Iface', 'no IPv4 routes')
    ipv6 = read(f'/proc/{pid}/net/ipv6_route', deadline)
    for line in ipv6.splitlines():
        row = line.split()
        require(len(row) == 10 and row[-1] == 'lo' and row[0] in ['0' * 32, '0' * 31 + '1']
                and row[1] in ['00', '80'] and row[2] == '0' * 32 and row[3] == '00'
                and row[4] == '0' * 32, 'no IPv6 external destination or next hop')
    observable = all(links[x + '/net']['actualLink'] is not None for x in ['self', str(pid)])
    return {'interfaces': devices, 'ipv4Routes': route, 'ipv6Routes': ipv6,
            'privateNamespaceLinkVerified': observable, 'namespaceIdentityUnobserved': not observable,
            'meaning': 'Actual only-loopback/no-external-routes; unavailable namespace links are not guessed.'}
