"""Real Windows filesystem controls; authored only, never run by source author."""
import ctypes,importlib.util,os,tempfile,time,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('fixed_native',Path(__file__).with_name('windows_file_identity.py'))
W=importlib.util.module_from_spec(spec);spec.loader.exec_module(W)

@unittest.skipUnless(os.name=='nt','Actual Windows native API required; no Windows credit here')
class NativeIdentityControls(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=Path(self.tmp.name);self.path=self.root/'archive.zip';self.path.write_bytes(b'original')
  self.fd=os.open(self.path,os.O_RDONLY|getattr(os,'O_BINARY',0));self.addCleanup(os.close,self.fd)
 def capture(self,path=None,fd=None):
  return W.capture(self.fd if fd is None else fd,self.path if path is None else path,8,time.monotonic()+5.0)
 def test_stable_physical_identity_and_both_native_times(self):
  before=self.capture();after=self.capture();W.require_same(before,after)
  self.assertEqual(set(before),{'volumeSerial','fileId128','size','allocation','links',
   'attributes','creationTime','lastWriteTime','changeTime'})
  self.assertEqual(before['links'],'1');self.assertNotEqual(before['fileId128'],'00'*16)
 def test_same_bytes_foreign_named_inode_rejected(self):
  other=self.root/'other.zip';other.write_bytes(b'original')
  with self.assertRaises(W.IdentityError):self.capture(other)
 def test_named_swap_rejected_while_original_fd_retained(self):
  # Separate literal share-delete original; CRT fixture FD cannot guarantee share-delete.
  import msvcrt
  path=self.root/'swap.zip';path.write_bytes(b'original');native=W.Native()
  handle=native.create(str(path),0x80000080,7,None,3,0x00200000,None)
  if handle in (None,ctypes.c_void_p(-1).value):raise ctypes.WinError(ctypes.get_last_error())
  try:fd=msvcrt.open_osfhandle(handle,os.O_RDONLY|os.O_BINARY)
  except BaseException:
   native.close(handle);raise
  self.addCleanup(os.close,fd);before=self.capture(path,fd)
  path.rename(self.root/'retained.zip');path.write_bytes(b'original')
  with self.assertRaises(W.IdentityError):self.capture(path,fd)
  self.assertEqual(before['links'],'1')
 def test_hardlink_to_retained_file_rejected(self):
  os.link(self.path,self.root/'alias.zip')
  with self.assertRaises(W.IdentityError):self.capture()
 def test_same_size_write_restored_mtime_still_rejected_by_change_time(self):
  before=self.capture();old=os.stat(self.path)
  with self.path.open('r+b') as output:
   output.write(b'changed!');output.flush();os.fsync(output.fileno())
  os.utime(self.path,ns=(old.st_atime_ns,old.st_mtime_ns))
  after=self.capture()
  self.assertEqual(before['fileId128'],after['fileId128'])
  self.assertEqual(before['lastWriteTime'],after['lastWriteTime'])
  self.assertNotEqual(before['changeTime'],after['changeTime'])
  with self.assertRaises(W.IdentityError):W.require_same(before,after)

if __name__=='__main__':unittest.main()
