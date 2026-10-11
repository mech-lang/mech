#!/usr/bin/env python3
"""New byte operations compose with prior native windows/histograms."""
import json,tempfile
from pathlib import Path
import verify
source=r'''encoded := bytes/encode("ACGTACGT", "ACGT", "exact")
r := sequence/reverse(encoded)
io/write(number/format(stats/sum(map/values(map/histogram(sequence/packed-windows(r,2,2)))),0))
t := bytes/table("ACGT", "\0\u{1}\u{2}\u{3}", "error")
a := bytes/translate(bytes/from-text("ACGTACGT"),t)
io/write(number/format(stats/sum(map/values(map/histogram(sequence/packed-windows(a,2,2)))),0))
'''
with tempfile.TemporaryDirectory() as tmp:
 for simd in (False,True):
  b=verify.build(source,Path(tmp)/('compose'+str(simd)),simd=simd)
  r=verify.execute(b,b'',3);assert r.returncode==0 and r.stdout==b'77',(r.stderr,r.stdout)
(verify.run.HERE/'results/byte-domain-composition.json').write_text(json.dumps({'all_passed':True,'reversal_preserves_encoding_domain':True,'translation_infers_output_domain':True,'packed_windows_histograms_reused':True,'scalar_and_simd':True,'expected_output':'77'},indent=2)+'\n')
print('New byte operations compose with existing packed windows and histograms')
