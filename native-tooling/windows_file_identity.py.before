"""Windows transport-only native identity. No Python stat/fstat timestamp comparison."""
import ctypes,os,time
from pathlib import Path

class IdentityError(RuntimeError):
 pass

def need(ok,message):
 if not ok:raise IdentityError(message)

class Basic(ctypes.Structure):
 _fields_=[('creation',ctypes.c_int64),('access',ctypes.c_int64),
  ('write',ctypes.c_int64),('change',ctypes.c_int64),('attributes',ctypes.c_uint32)]

class Standard(ctypes.Structure):
 _fields_=[('allocation',ctypes.c_int64),('size',ctypes.c_int64),
  ('links',ctypes.c_uint32),('delete',ctypes.c_ubyte),('directory',ctypes.c_ubyte)]

class FileId(ctypes.Structure):
 _fields_=[('volume',ctypes.c_uint64),('identity',ctypes.c_ubyte*16)]

class Native:
 def __init__(self):
  need(os.name=='nt' and ctypes.sizeof(ctypes.c_void_p)==8,'Windows x64 only')
  need((ctypes.sizeof(Basic),Basic.attributes.offset)==(40,32),'FILE_BASIC_INFO ABI')
  need((ctypes.sizeof(Standard),Standard.links.offset,Standard.delete.offset,
   Standard.directory.offset)==(24,16,20,21),'FILE_STANDARD_INFO ABI')
  need((ctypes.sizeof(FileId),FileId.identity.offset)==(24,8),'FILE_ID_INFO ABI')
  k=ctypes.WinDLL('kernel32',use_last_error=True)
  self.create=k.CreateFileW
  self.create.argtypes=[ctypes.c_wchar_p,ctypes.c_uint32,ctypes.c_uint32,
   ctypes.c_void_p,ctypes.c_uint32,ctypes.c_uint32,ctypes.c_void_p]
  self.create.restype=ctypes.c_void_p
  self.info=k.GetFileInformationByHandleEx
  self.info.argtypes=[ctypes.c_void_p,ctypes.c_int,ctypes.c_void_p,ctypes.c_uint32]
  self.info.restype=ctypes.c_int
  self.file_type=k.GetFileType
  self.file_type.argtypes=[ctypes.c_void_p];self.file_type.restype=ctypes.c_uint32
  self.close=k.CloseHandle
  self.close.argtypes=[ctypes.c_void_p];self.close.restype=ctypes.c_int
 def check(self,deadline):
  need(type(deadline) is float and time.monotonic()<deadline,'Original extraction deadline')
 def read(self,handle,expected_bytes,deadline):
  self.check(deadline)
  need(self.file_type(handle)==1,'Native disk file only')
  values=[]
  for kind,shape in ((0,Basic),(1,Standard),(18,FileId)):
   self.check(deadline);value=shape()
   if not self.info(handle,kind,ctypes.byref(value),ctypes.sizeof(value)):
    raise ctypes.WinError(ctypes.get_last_error())
   values.append(value)
  self.check(deadline);basic,standard,identity=values
  need(not basic.attributes & (0x400|0x10|0x40),'No reparse/directory/device')
  need(standard.delete==0 and standard.directory==0,'No deleting/directory file')
  need(standard.links==1 and standard.size==expected_bytes,'Native single-link exact length')
  file_id=bytes(identity.identity).hex();need(file_id!='00'*16,'Nonzero 128-bit native file ID')
  # AccessTime intentionally excluded: original contract did not guard read-induced atime.
  return {'volumeSerial':str(identity.volume),'fileId128':file_id,
   'size':str(standard.size),'allocation':str(standard.allocation),'links':str(standard.links),
   'attributes':str(basic.attributes),'creationTime':str(basic.creation),
   'lastWriteTime':str(basic.write),'changeTime':str(basic.change)}

def require_same(before,after):
 need(type(before) is dict and type(after) is dict and before==after,
  'Exact native identity/creation/write/change metadata unchanged')

def capture(fd,path,expected_bytes,deadline):
 import msvcrt
 need(type(fd) is int and type(expected_bytes) is int and expected_bytes>0,'Fixed transport FD/length')
 path=Path(path);need(path.is_absolute(),'Absolute original transport name')
 native=Native();borrowed=msvcrt.get_osfhandle(fd)
 need(borrowed not in (-1,0),'Valid borrowed original FD handle')
 before=native.read(borrowed,expected_bytes,deadline);native.check(deadline)
 # FILE_READ_ATTRIBUTES, share R/W/DELETE, OPEN_EXISTING, OPEN_REPARSE_POINT.
 named=native.create(str(path),0x80,7,None,3,0x00200000,None)
 if named in (None,ctypes.c_void_p(-1).value):raise ctypes.WinError(ctypes.get_last_error())
 try:
  current=native.read(named,expected_bytes,deadline)
  after=native.read(borrowed,expected_bytes,deadline)
  require_same(before,current);require_same(before,after)
  return before
 finally:
  if not native.close(named):raise ctypes.WinError(ctypes.get_last_error())
