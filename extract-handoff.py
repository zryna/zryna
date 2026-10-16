"""Fixed authenticated transport ZIP members only; never execute extracted payloads."""
import hashlib,json,os,stat,sys,time,zipfile,zlib
from pathlib import Path
need=lambda ok,message: None if ok else (_ for _ in ()).throw(RuntimeError(message))
EXPECTED=(55958883,'01f566e79945b4a1108fd91b308f975aa57d2191ed86f6457d05a8319ad02871') # UNBOUND: exact newly selected current Linux transport bytes/SHA
need(type(EXPECTED[0]) is int and 0<EXPECTED[0]<=128*1024*1024 and
 isinstance(EXPECTED[1],str) and len(EXPECTED[1])==64 and
 all(c in '0123456789abcdef' for c in EXPECTED[1]),'UNBOUND current Linux transport identity')
need(len(sys.argv)==4,'Fixed transport/destination/platform archive arguments')
archive,destination,name=map(Path,sys.argv[1:])
need(archive.is_absolute() and destination.is_absolute(),'Absolute diagnostic paths')
need(archive.name=='handoff.transport.zip' and destination.name=='m6-editor-handoff','Fixed filenames')
allowed={'zryna-setup-0.1.0-candidate.3-x86_64-unknown-linux-gnu.tar.gz'}
need(str(name) in allowed,'Fixed selected platform archive')
need(not archive.is_symlink() and archive.is_file(),'Ordinary verified archive')
need(not destination.exists() and not destination.is_symlink(),'Fresh destination')
CAP=128*1024*1024;end=time.monotonic()+25;total=0;rows=[]
expected=EXPECTED
fields=('st_dev','st_ino','st_mode','st_size','st_nlink','st_mtime_ns','st_ctime_ns')
fd=os.open(archive,os.O_RDONLY|getattr(os,'O_NOFOLLOW',0)|getattr(os,'O_BINARY',0))
transport=os.fdopen(fd,'rb');before=os.fstat(transport.fileno())
need(stat.S_ISREG(before.st_mode) and before.st_nlink==1 and before.st_size==expected[0],
 'Exact single-link transport identity')
count=0;h=hashlib.sha256()
while True:
 need(time.monotonic()<end,'Original extraction deadline')
 part=transport.read(65536)
 if not part:break
 count+=len(part);need(count<=expected[0],'Transport first-extra byte');h.update(part)
need((count,h.hexdigest())==expected,'Retained full transport bytes/SHA before ZIP parse')
transport.seek(0)
with zipfile.ZipFile(transport) as z:
 infos=z.infolist();names={'candidate-receipt.json','reproduction.json',str(name)}
 need(len(infos)==3 and {i.filename for i in infos}==names,'Exact three distinct regular members')
 destination.mkdir(mode=0o700)
 for i in infos:
  need(time.monotonic()<end,'Original extraction deadline')
  need(not i.is_dir() and stat.S_IFMT(i.external_attr>>16) in (0,stat.S_IFREG)
   and not(i.flag_bits&1) and i.compress_type in (zipfile.ZIP_STORED,zipfile.ZIP_DEFLATED),
   'Regular nonencrypted stored/deflate member only')
  limit=262144 if i.filename.endswith('.json') else CAP
  need(0<i.file_size<=limit and total+i.file_size<=CAP,'Bounded member/aggregate')
  count=0;crc=0;h=hashlib.sha256()
  with z.open(i) as source,open(destination/i.filename,'xb') as output:
   while True:
    need(time.monotonic()<end,'Original extraction deadline')
    chunk=source.read(min(65536,limit-count+1))
    if not chunk:break
    count+=len(chunk);need(count<=limit and count<=i.file_size,'First-extra member byte refused')
    crc=zlib.crc32(chunk,crc);h.update(chunk);output.write(chunk)
   output.flush();os.fsync(output.fileno())
  need(count==i.file_size and crc==i.CRC,'Full original member length/CRC')
  total+=count;rows.append({'path':i.filename,'bytes':count,'sha256':h.hexdigest(),'crc32':crc})
after=os.fstat(transport.fileno());named=os.stat(archive,follow_symlinks=False)
need(all(getattr(before,k)==getattr(after,k)==getattr(named,k) for k in fields),
 'Retained and named original transport identity after complete extraction')
transport.close()
need(time.monotonic()<end,'No positive completion after deadline')
print(json.dumps({'members':rows,'expandedBytes':total,'crcVerified':True},separators=(',',':')))
