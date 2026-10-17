"""SOURCE ONLY, UNRUN. Exercise Windows fingerprint branch with mocked native snapshots.
No child, native Win32 API, network or extracted payload execution by these controls.
"""
import hashlib,importlib.util,tempfile,time,unittest
from pathlib import Path
from unittest.mock import patch

def load(name,filename):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(filename))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

R=load('fixed_wrapper_v2','run-fixed.py')
N=load('fixed_native_v2','windows_file_identity.py')

class WindowsBranchControls(unittest.TestCase):
    def run_branch(self,first,second):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'empty.stdout';path.write_bytes(b'')
            with patch.object(R,'WINDOWS',True),patch.object(R,'NATIVE_IDENTITY',N), \
                    patch.object(N,'capture',side_effect=[first,second]) as capture:
                actual=R.fingerprint(path,time.monotonic()+5.0)
                self.assertEqual(capture.call_count,2)
                for call in capture.call_args_list:
                    self.assertEqual(call.args[1],path);self.assertEqual(call.args[2],0)
                return actual
    def identity(self):
        return {'volumeSerial':'7','fileId128':'01'*16,'size':'0','allocation':'0','links':'1',
                'attributes':'32','creationTime':'100','lastWriteTime':'200','changeTime':'300'}
    def test_stable_native_empty_stream_returns_exact_native_identity(self):
        identity=self.identity();actual=self.run_branch(identity,dict(identity))
        self.assertEqual(actual['bytes'],0)
        self.assertEqual(actual['sha256'],hashlib.sha256(b'').hexdigest())
        self.assertEqual(actual['identity'],identity)
        self.assertNotIn('changedNs',actual['identity'])
    def test_same_hash_different_native_file_id_refused(self):
        before=self.identity();after={**before,'fileId128':'02'*16}
        with self.assertRaises(N.IdentityError):self.run_branch(before,after)
    def test_same_hash_native_change_time_mutation_refused(self):
        before=self.identity();after={**before,'changeTime':'301'}
        with self.assertRaises(N.IdentityError):self.run_branch(before,after)

    def test_invalid_or_expired_deadline_refuses_before_native_capture(self):
        for deadline in (None,float('inf'),float('nan'),time.monotonic()-1.0):
            with self.subTest(deadline=deadline),tempfile.TemporaryDirectory() as directory:
                path=Path(directory)/'empty.stdout';path.write_bytes(b'')
                with patch.object(R,'WINDOWS',True),patch.object(R,'NATIVE_IDENTITY',N), \
                        patch.object(N,'capture') as capture:
                    with self.assertRaises(AssertionError):R.fingerprint(path,deadline)
                    capture.assert_not_called()

    def test_last_point_after_stable_native_snapshots_and_file_close_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'empty.stdout';path.write_bytes(b'')
            identity=self.identity()
            with patch.object(R,'WINDOWS',True),patch.object(R,'NATIVE_IDENTITY',N), \
                    patch.object(N,'capture',side_effect=[identity,dict(identity)]) as capture, \
                    patch.object(R.time,'monotonic',side_effect=[1.0,3.0]) as clock:
                with self.assertRaisesRegex(AssertionError,'Original fingerprint observation deadline'):
                    R.fingerprint(path,2.0)
                self.assertEqual(capture.call_count,2)
                self.assertEqual(clock.call_count,2)

if __name__=='__main__':unittest.main()
