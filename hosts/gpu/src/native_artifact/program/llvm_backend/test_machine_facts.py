"""Exercise actual LLVM Objects and refuse incomplete machine metadata."""
from pathlib import Path
import importlib.util,json,os,struct,subprocess,tempfile,unittest

P=Path(__file__).parent
spec=importlib.util.spec_from_file_location('machine_facts',P/'machine_facts.py');facts=importlib.util.module_from_spec(spec);spec.loader.exec_module(facts)
LLC=Path(os.environ.get('LLVM_LLC','/opt/homebrew/opt/llvm/bin/llc'))

@unittest.skipUnless(LLC.is_file(),'LLVM machine emitter required')
class MachineFacts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory=tempfile.TemporaryDirectory(prefix='mech-llvm-machine-facts-');p=Path(cls.directory.name)
        (p/'input.ll').write_text('''target triple = "arm64-apple-macosx15.0.0"
declare void @escape(ptr)
define i64 @leaf(i64 %a) #0 { %r=add i64 %a,7
ret i64 %r }
define double @floating(double %a,double %b) #0 { %r=fadd double %a,%b
ret double %r }
define i64 @with_stack(i64 %a) #0 {
%p=alloca [64 x i64],align 16
store i64 %a,ptr %p,align 16
call void @escape(ptr %p)
%r=load i64,ptr %p,align 16
ret i64 %r }
attributes #0 = { nounwind noredzone "frame-pointer"="none" }
''')
        r=subprocess.run([str(LLC),'-O=3','-global-isel=0','-mcpu=apple-m3','-filetype=obj','-pass-remarks-output='+str(p/'remarks.yaml'),str(p/'input.ll'),'-o',str(p/'code.o')],capture_output=True,text=True)
        if r.returncode:raise RuntimeError(r.stderr)
        cls.raw=(p/'code.o').read_bytes();cls.remarks=(p/'remarks.yaml').read_text();cls.names=['leaf','floating','with_stack']
    @classmethod
    def tearDownClass(cls):cls.directory.cleanup()
    def test_actual_machine_frames_and_code(self):
        result=facts.extract(self.raw,self.remarks,self.names)
        self.assertEqual(result['functions']['leaf']['stack_bytes'],0)
        self.assertEqual(result['functions']['floating']['stack_bytes'],0)
        self.assertGreaterEqual(result['functions']['with_stack']['stack_bytes'],512)
        self.assertEqual(result['functions']['with_stack']['stack_bytes']%16,0)
        self.assertGreater(result['code_bytes'],0)
        self.assertEqual(result['text_bytes'],sum(x['code_bytes']for x in result['functions'].values())+result['padding_before_first_function_bytes'])
        self.assertFalse(result['source_or_layout_authority_granted'])
    def test_missing_stack_report_refused(self):
        with self.assertRaises(ValueError):facts.extract(self.raw,self.remarks.replace('Name:            StackSize','Name:            Other',1),self.names)
    def test_extra_stack_report_refused(self):
        with self.assertRaises(ValueError):facts.extract(self.raw,self.remarks,['leaf','floating'])
    def test_duplicate_stack_report_refused(self):
        with self.assertRaises(ValueError):facts.extract(self.raw,self.remarks+self.remarks,self.names)
    def test_duplicate_definition_manifest_refused(self):
        with self.assertRaises(ValueError):facts.extract(self.raw,self.remarks,self.names+['leaf'])
    def test_wrong_target_or_object_kind_refused(self):
        for offset,value in [(0,0),(4,0x01000007),(8,2),(12,2)]:
            raw=bytearray(self.raw);struct.pack_into('<I',raw,offset,value)
            with self.subTest(offset=offset),self.assertRaises(ValueError):facts.extract(bytes(raw),self.remarks,self.names)
    def test_truncated_object_refused(self):
        for size in [0,16,32,len(self.raw)-1]:
            with self.subTest(size=size),self.assertRaises(ValueError):facts.extract(self.raw[:size],self.remarks,self.names)

if __name__=='__main__':unittest.main()
