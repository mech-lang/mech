"""Use matrix data directly, avoiding named records and redundant state copies."""
from pathlib import Path
import re
HERE=Path(__file__).resolve().parent
S=HERE/'sources'
original=(S/'nbody.bqn').read_text()
fast=(S/'nbody-flat-cached-reduce.bqn').read_text()
initial='''# Pure BQN n-body; standard five-body inputs, one matrix row per line.
solar_mass ← 4×3.14159265358979323×3.14159265358979323
pxyz ← 3‿5⥊⟨
  0, 4.84143144246472090e00, 8.34336671824457987e00, 1.28943695621391310e01, 1.53796971148509165e01,
  0, -1.16032004402742839e00, 4.12479856412430479e00, -1.51111514016986312e01, -2.59193146099879641e01,
  0, -1.03622044471123109e¯01, -4.03523417114321381e¯01, -2.23307578892655734e¯01, 1.79258772950371181e¯01⟩
vxyz ← 365.24×3‿5⥊⟨
  0, 1.66007664274403694e¯03, -2.76742510726862411e¯03, 2.96460137564761618e¯03, 2.68067772490389322e¯03,
  0, 7.69901118419740425e¯03, 4.99852801234917238e¯03, 2.37847173959480950e¯03, 1.62824170038242295e¯03,
  0, -6.90460016972063023e¯05, 2.30417297573763929e¯05, -2.96589568540237556e¯05, -9.51592254519715870e¯05⟩
masses ← solar_mass×⟨1, 9.54791938424326609e¯04, 2.85885980666130812e¯04, 4.36624404335156298e¯05, 5.15138902046611451e¯05⟩
momentum ← -(+˝˘vxyz×⎉1 masses)÷solar_mass
vxyz momentum⌾(⊏˘)↩
'''
energy='''Energy ← {𝕊:
  (0.5×+´masses×+˝×˜vxyz) - +´+´¨(↕∘≠↑¨<˘)(×⌜˜masses)÷√+˝⎉1×˜> -⌜˜<˘⍉pxyz}
'''
formatter=next(l.strip() for l in original.splitlines() if 'Fmt9 ←' in l)
advance=original.split('diags ←',1)[1].split('\n\n· ←',1)[0]
compact=initial+'\n'+energy+'\ndiags ←'+advance.replace('≠system','≠masses')+'\n\n'+formatter+'\nFmt9 Energy@\nAdvance∘0.01⍟(•ParseFloat ⋈⁼•args) @\nFmt9 Energy@\n'
(S/'nbody-compact.bqn').write_text(compact)
# Optimized variant: store the same scaled flat state once and use it for both energy and advancement.
init=initial.replace('pxyz ← 3‿5⥊⟨','p ← ⟨').replace('vxyz ← 365.24×3‿5⥊⟨','v ← 0.01×365.24×⟨')
init=init.split('momentum ←',1)[0]
setup=fast.split('pair_count ←',1)[1].split('p ← ⥊pxyz',1)[0]
setup='pair_count ←'+setup
adv=fast.split('Advance ← ',1)[1].split('\n\n· ←',1)[0]
energy_fast='''Energy ← {𝕊:
  (0.5×+´masses×(+˝3‿pair_count⥊v×v)÷0.01×0.01) - +´((pair_i⊏masses)×pair_j⊏masses)÷√+˝3‿pair_size⥊×˜(position_i⊏p)-position_j⊏p}
'''
optimized=init+'\n'+setup+'momentum ← -(+˝˘3‿pair_count⥊v×∾3⥊<masses)÷solar_mass\nv momentum⌾((pair_count×↕3)⊸⊏)↩\n\n'+energy_fast+'\nAdvance ← '+adv+'\n\n'+formatter+'\nFmt9 Energy@\nAdvance⍟(•ParseFloat ⋈⁼•args) @\nFmt9 Energy@\n'
(S/'nbody-optimized-short.bqn').write_text(optimized)
# K: shorten only initialization/reporting. The measured fast advance function is unchanged.
k=(S/'nbody.growler.k').read_text()
k=k.replace('solarmass:4*/2#3.14159265358979323;daysperyear:365.24;dt:0.01','dt:0.01')
k=k.replace('*daysperyear*dt','*365.24*dt').replace('*solarmass*dt*dt','*(4*/2#3.14159265358979323)*dt*dt')
k=k.replace(';V0:!5;V1:5+V0;V2:10+V0','')
k=k.replace('report:{d:(p P0)-p P1;dd:d*d;vv:v*v\n        E:(1%dt*dt*dt*dt)*(0.5*+/m*(vv V0)+(vv V1)+vv V2)-+/(m[I]*m J)%%(dd D0)+(dd D1)+dd D2',
 'report:{d:(p P0)-p P1\n        E:(1%dt*dt*dt*dt)*(0.5*+/m*+/3 5#v*v)-+/(m[I]*m J)%%+/3 10#d*d')
k=k.replace("(-{+/m*x}'(v V0;v V1;v V2))%*m", "(-{+/m*x}'3 5#v)%*m")
(S/'nbody-optimized-short.k').write_text(k)
for name in ['nbody-compact.bqn','nbody-optimized-short.bqn','nbody-optimized-short.k']:
 s=(S/name).read_text()
 markers=('#',) if name.endswith('bqn') else ('/','#!')
 print(name,len([l for l in s.splitlines() if l.strip() and not l.lstrip().startswith(markers)]),'physical LOC')
# Expand K's sequential and nested assignments into executable physical lines.
replacements={
 '(I;J):&~|\\=n:#m':'n:#m\n(I;J):&~|\\=n',
 'P0:,/I+/:0 5 10;P1:,/J+/:0 5 10':'P0:,/I+/:0 5 10\nP1:,/J+/:0 5 10',
 'D0:!10;D1:10+D0;D2:20+D0;Ni:10!!30':'D0:!10\nD1:10+D0\nD2:20+D0\nNi:10!!30',
 'ks:{(&I=x),30+&J=x}\'!n;(F0;F1;F2;F3):+,/ks+/:0 10 20':'ks:{(&I=x),30+&J=x}\'!n\n(F0;F1;F2;F3):+,/ks+/:0 10 20',
 'adv:{[z]d:(p P0)-p P1;dd:d*d;N:(dd D0)+(dd D1)+dd D2;fv:(a,a:d%(N*%N)Ni)*Mf;v+:(fv F0)+(fv F1)+(fv F2)+fv F3;p+:v}':
 'adv:{[z]\n d:(p P0)-p P1\n dd:d*d\n N:(dd D0)+(dd D1)+dd D2\n a:d%(N*%N)Ni\n fv:(a,a)*Mf\n v+:(fv F0)+(fv F1)+(fv F2)+fv F3\n p+:v}',
 'report:{d:(p P0)-p P1':'report:{\n d:(p P0)-p P1',
 'r:$_0.5+1e9*E;`0:':'r:$_0.5+1e9*E\n `0:',
 'report 0;(. `argv 2)adv/0;report 0':'report 0\n(. `argv 2)adv/0\nreport 0',
}
for old,new in replacements.items():
 assert old in k,old
 k=k.replace(old,new)
(S/'nbody-optimized-short.k').write_text(k)
assert len([l for l in k.splitlines() if l.strip() and not l.lstrip().startswith(('/','#!'))])==36
print('K rewritten into 36 executable normalized physical LOC.')
