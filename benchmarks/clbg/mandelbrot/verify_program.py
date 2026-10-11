#!/usr/bin/env python3
"""Independent, non-Mandelbrot checks for the compiled Mech grid operation."""
import hashlib,json,os,pathlib,subprocess,tempfile
HERE=pathlib.Path(__file__).resolve().parent
ROOT=HERE.parents[2]
DRIVER=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech-clbg-mandelbrot-compiled'
SDK=subprocess.run(["xcrun","--show-sdk-path"],check=True,capture_output=True,text=True).stdout.strip()
CLANG='/opt/homebrew/opt/llvm@22/bin/clang-22'
def run(argv,**kw):return subprocess.run([str(x) for x in argv],check=True,capture_output=True,**kw)
checks=[]
with tempfile.TemporaryDirectory(prefix='mech-grid-conformance-') as directory:
 temp=pathlib.Path(directory)
 for lanes in (1,2,7,8,9,63,64):
  kind='<f64>' if lanes==1 else f'<[f64]:{lanes},1>'
  flags='flags' if lanes==1 else ', '.join(f'flags[{i},1]' for i in range(1,lanes+1))
  # A stateful diagonal half-plane, with no Mandelbrot algorithm or constants.
  source=f'''pixels(width<f64>, column{kind}, row<f64>) = (output<[bool]:1,{lanes+1}>) :=
  ~turn := 0.0
  flags := column + row + turn < width
  done := turn >= 3.0
  turn += 1.0
  output := [{flags}, done].
width := io/argument-u64(1)
height := io/argument-u64(2)
image := grid/map-until(width, height, pixels)
io/write(image/pbm(image))
'''
  src=temp/'grid.mec';src.write_text(source)
  for simd in (False,True):
   c,obj,exe=(temp/n for n in ('grid.c','grid.o','grid'))
   run([DRIVER,src,'emit-cranelift-simd' if simd else 'emit-cranelift',c,obj,'--source-program'])
   run([CLANG,'-O3','-mcpu=native','-ffp-contract=off','-isysroot',SDK,c,obj,'-lm','-pthread','-o',exe])
   for width in (1,7,8,9,13,63,64,65,129):
    for height,threads in ((3,1),(5,4)):
     expected=bytearray(f'P4\n{width} {height}\n'.encode());stride=(width+7)//8
     for row in range(height):
      bits=bytearray(stride)
      for column in range(width):
       if column+row+3<width:bits[column//8]|=128>>(column%8)
      expected.extend(bits)
     result=run([exe,width,height,threads]).stdout
     assert result==expected,(lanes,simd,width,height)
     checks.append({'lanes':lanes,'simd':simd,'width':width,'height':height,'threads':threads,'bytes':len(result),'sha256':hashlib.sha256(result).hexdigest()})
   # Missing dimensions and malformed/zero dimensions cannot reach native code.
   for argv in ([],['9'],['9','0'],['bad','3'],['9','3','0']):
    result=subprocess.run([str(exe),*argv],capture_output=True)
    assert result.returncode==2
   print(f'lanes={lanes} SIMD={simd}: 18 rectangular images, state reset, tails and argument errors passed',flush=True)
(HERE/'results/revised-grid-conformance.json').write_text(json.dumps({'kernel':'stateful diagonal half-plane; independent of Mandelbrot','images':checks,'exact_byte_output':True,'argument_errors_rejected':True},indent=2)+'\n')
