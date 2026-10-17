"""Fixed authenticated transport ZIP members only; never execute extracted payloads."""
import hashlib,importlib.util,json,os,stat,sys,time,zipfile,zlib
from pathlib import Path
need=lambda ok,message: None if ok else (_ for _ in ()).throw(RuntimeError(message))
need(len(sys.argv)==4,'Fixed transport/destination/platform archive arguments')
archive,destination,name=map(Path,sys.argv[1:])
need(archive.is_absolute() and destination.is_absolute(),'Absolute diagnostic paths')
need(archive.name=='handoff.transport.zip' and destination.name=='m6-editor-handoff','Fixed filenames')
allowed={'zryna-setup-0.1.0-candidate.3-x86_64-unknown-linux-gnu.tar.gz',
 'zryna-setup-0.1.0-candidate.3-x86_64-pc-windows-msvc.zip'}
need(str(name) in allowed,'Fixed selected platform archive')
need(not archive.is_symlink() and archive.is_file(),'Ordinary verified archive')
need(not destination.exists() and not destination.is_symlink(),'Fresh destination')
CAP=128*1024*1024;end=time.monotonic()+25;total=0;rows=[]
expected={
 'zryna-setup-0.1.0-candidate.3-x86_64-unknown-linux-gnu.tar.gz':
  (55958902,'e4e182edb6f93f93bd9dba8784a72fd4e3f6d2aedf0030d43ece68cbe6608df8'),
 'zryna-setup-0.1.0-candidate.3-x86_64-pc-windows-msvc.zip':
  (125331054,'8043d2bd89307b8d5096b844cdaf6f475a0e1c1e0bb69c0504d54a17d26da8a4')}[str(name)]
fields=('st_dev','st_ino','st_mode','st_size','st_nlink','st_mtime_ns','st_ctime_ns')
fd=os.open(archive,os.O_RDONLY|getattr(os,'O_NOFOLLOW',0)|getattr(os,'O_BINARY',0))
transport=os.fdopen(fd,'rb');before=os.fstat(transport.fileno())
need(stat.S_ISREG(before.st_mode) and before.st_nlink==1 and before.st_size==expected[0],
 'Exact single-link transport identity')
native_identity=None;native_before=None;native_after=None
if os.name=='nt':
 helper=Path(__file__).with_name('windows_file_identity.py')
 need(helper.is_file() and not helper.is_symlink() and helper.stat().st_nlink==1,'Ordinary fixed Windows identity helper')
 body=helper.read_bytes();need(len(body)<=65536 and hashlib.sha256(body).hexdigest()=='9c41d5954bde66a3e047bc3b3fc8f475848c07a3a474a4f6606254536669b680','Exact reviewed native identity helper')
 spec=importlib.util.spec_from_file_location('fixed_windows_identity',helper)
 native_identity=importlib.util.module_from_spec(spec);spec.loader.exec_module(native_identity)
 native_before=native_identity.capture(transport.fileno(),archive,expected[0],end)
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
observed={'pythonBefore':{k:getattr(before,k) for k in fields},
 'pythonAfter':{k:getattr(after,k) for k in fields},'pythonNamed':{k:getattr(named,k) for k in fields}}
if native_identity:
 native_after=native_identity.capture(transport.fileno(),archive,expected[0],end)
 observed.update(nativeBefore=native_before,nativeAfter=native_after)
with open(archive.parent/'IDENTITY_OBSERVATIONS.json','x',encoding='utf-8') as output:
 json.dump(observed,output,indent=2);output.flush();os.fsync(output.fileno())
if native_identity:native_identity.require_same(native_before,native_after)
else:need(all(getattr(before,k)==getattr(after,k)==getattr(named,k) for k in fields),
 'Retained and named original transport identity after complete extraction')
transport.close()
need(time.monotonic()<end,'No positive completion after deadline')
print(json.dumps({'members':rows,'expandedBytes':total,'crcVerified':True},separators=(',',':')))
