from pathlib import Path
D=Path(__file__).resolve().parent
S=D/'sources/uiua'
FIX='Fixed ← /◇⊂ {⊃(↘¯9|⋅"."|↙¯9)} ⬚@0↙¯10 °⋕ ⁅×1e9\n'
def save(b,v,text):
 p=S/b/(v+'.ua');p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
# The coefficient array is rebuilt inside each matrix/vector product. No
# coefficient matrix is retained between the 40 canonical A/transpose products.
spec='''N ← ⋕◇∘⊡1 &args
A ← ÷⊙1 +1 +⟜(÷2×⊸+1+)
Forward ← ≡/+× (˙⊞A ⇡N) ¤
Transpose ← ≡/+× ⍉(˙⊞A ⇡N) ¤
Step ← Transpose Forward
V ← ⍥Step 19 ↯N 1
U ← Step V
'''+FIX+'''&p Fixed √ ÷(/+˙×V) /+×U V
'''
save('spectralnorm','compact',spec);save('spectralnorm','fast',spec)
nb='''N ← ⋕◇∘⊡1 &args
Solar ← ×4˙×π
Masses ← ×Solar [1 0.000954791938424326609 0.000285885980666130812 0.0000436624404335156298 0.0000515138902046611451]
Positions ← [0_0_0 4.84143144246472090_¯1.16032004402742839_¯0.103622044471123139 8.34336671824457987_4.12479856412430479_¯0.403523417114321381 12.8943695621391310_¯15.1111514016986312_¯0.223307578892655734 15.3796971148509165_¯25.9193146099879641_0.179258772950371181]
Velocities ← ×365.24 [0_0_0 0.00166007664274403694_0.00769901118419740425_¯0.0000690460016972063023 ¯0.00276742510726862411_0.00499852801234917238_0.0000230417297573763929 0.00296460137564761618_0.00237847173959480950_¯0.0000296589568540237556 0.00268067772490389322_0.00162824170038242295_¯0.0000951592254519715870]
Initial ← ≡⊂ Positions ⊂¤(¯÷Solar/+×⍉¤Masses Velocities) ↘1 Velocities
Pos ← ≡↙3
Vel ← ≡↘3
Differences ← ˙⊞- Pos
Norm ← ≡≡/+˙×
Diagonal ← ˙⊞= ⇡5
Upper ← ˙⊞> ⇡5
Acceleration ← ≡/+ ÷⊃(≡≡(¤√ⁿ3)+Diagonal Norm)(×↯1_5_1 Masses) Differences
NewVelocity ← +⊃Vel(×0.01 Acceleration)
Advance ← ≡⊂ ⊃(+⊃Pos(×0.01 NewVelocity))NewVelocity
Potential ← /+♭ ×Upper ÷⊃(√+Diagonal Norm Differences)(⋅(˙⊞×Masses))
Energy ← -⊃Potential(×0.5/+×Masses ≡/+˙×Vel)
'''+FIX+'''Print ← &p ⊂"-" Fixed ⌵ Energy
Print Initial
Print ⍥Advance N Initial
'''
save('nbody','compact',nb);save('nbody','fast',nb)
mb='''N ← ⋕◇∘⊡1 &args
Cr ← -1.5 ×2÷N⇡N
~M {R I Live Cx Cy Iter}
Advance ← M ⊃(+⊃M.Cx(-⊃(˙×M.I)(˙×M.R))|+⊃M.Cy(×2×⊃M.R M.I)|M.Live|M.Cx|M.Cy|+1M.Iter)
Mark ← M ⊃(M.R|M.I|×⊃M.Live(≤4+⊃(˙×M.R)(˙×M.I))|M.Cx|M.Cy|M.Iter)
Iterate ← Mark Advance
Pack ← ≡(/+×⇌˜ⁿ2⇡8) ⬚0↯(⌈÷8 N)_8
Row ← &fwa "/dev/stdout" Pack M.Live ⍥Iterate 50 M (↯N 0) (↯N 0) (↯N 1) Cr ⊙0
&p "P4"
&p $"_ _" N N
≡Row -1 ×2÷N⇡N
'''
save('mandelbrot','compact',mb)
save('mandelbrot','fast',mb.replace('⍥Iterate 50','⍢(Iterate|×⊃(<50M.Iter)(/↥M.Live))'))
rx='''Input ← &fras "/dev/stdin"
Initial ← ⧻Input
Clean ← ⍜regex≡(⋅"") ">.*\\n|\\n"
Sequence ← Clean Input
Length ← ⧻Sequence
Patterns ← {"agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]" "a[act]ggtaaa|tttacc[agt]t" "ag[act]gtaaa|tttac[agt]ct" "agg[act]taaa|ttta[agt]cct" "aggg[acg]aaa|ttt[cgt]ccct" "agggt[cgt]aa|tt[acg]accct" "agggta[cgt]a|t[acg]taccct" "agggtaa[cgt]|[acg]ttaccct"}
Count ← &p $"_ _" ⟜(⧻regex ⊙Sequence) ◇∘
≡Count Patterns
Replacements ← {"<4>" "<3>" "<2>" "|" "-"}
Substitutions ← {"tHa[Nt]" "aND|caN|Ha[DS]|WaS" "a[NSt]|BY" "<[^>]*>" "\\\\|[^|][^|]*\\\\|"}
Replace ← ⍜regex(≡⌟⋅∘) ⊃(◇∘⊡0|⋅∘|◇∘⊡1)
Final ← ∧Replace ≡⊟ Substitutions Replacements Sequence
&p ""
&p Initial
&p Length
&p ⧻Final
'''
save('regexredux','compact',rx);save('regexredux','fast',rx)
WRAP='Wrap ← ≡◇&p ⊕□⊸(⌊÷60⇡⧻)\n'
fa='''N ← ⋕◇∘⊡1 &args
Alu ← "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
Iub ← \\+[0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02]
Human ← \\+[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008]
'''+WRAP+'''~G {Seed Left Letters Cdf}
~Chunk {Draws State}
Size ← ↧60 G.Left
Next ← ◿139968 +29573×3877
Draw ← ⍥(⟜∘Next) ⊃Size G.Seed
Chars ← ⊏⊃(/+⊞≥⊃(G.Cdf Chunk.State)(÷139968 Chunk.Draws))(G.Letters Chunk.State)
Advance ← (
  ⟜(Wrap Chars)
  G ⊃(⊣Chunk.Draws|-⊃(⧻Chunk.Draws)(G.Left Chunk.State)|G.Letters Chunk.State|G.Cdf Chunk.State)
) Chunk ⊃Draw∘
Generate ← G.Seed ⍢(Advance|>0G.Left)
&p ">ONE Homo sapiens alu"
Wrap ↯[×2N] Alu
&p ">TWO IUB ambiguity codes"
Seed ← Generate G 42 (×3N) "acgtBDHKMNRSVWY" Iub
&p ">THREE Homo sapiens frequency"
◌ Generate G Seed (×5N) "acgt" Human
'''
save('fasta','compact',fa)
fast=fa.replace('Size ← ↧60 G.Left','Size ← ↧65520 G.Left').replace('Next ← ◿139968 +29573×3877\nDraw ← ⍥(⟜∘Next) ⊃Size G.Seed','Compose ← [⊃(◿139968×∩⊢)(◿139968+⊃⊣(×⊃⊢(⋅⊣)))]\nCoefficients ← \\Compose ↯65520_2 3877_29573\nSlice ← ↙⊃Size(⋅Coefficients)\nDraw ← ◿139968+⊃(×⊃G.Seed(≡⊢Slice))(≡⊣Slice)')
save('fasta','fast',fast)
rc='''Alphabet ← "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
Complement ← "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
'''+WRAP+'''Records ← regex "(?m)(>[^\\n]*\\n)([^>]*)" &fras "/dev/stdin"
Sequence ← ▽⊸(¬∊"\\r\\n") ◇∘⊡2
Record ← (
  ⟜(&pf◇∘⊡1)
  Wrap ⊏⊙Complement ⊗Alphabet ⇌Sequence
)
≡Record Records
'''
save('revcomp','compact',rc);save('revcomp','fast',rc.replace(WRAP,'Wrap ← &pf /◇⊂ ⊕(□⊂⊙"\\n")⊸(⌊÷60⇡⧻)\n'))
kn='''Input ← &fras "/dev/stdin"
Sequence ← ⊏⊙"ACGT" ◿4⊗"ACGTacgt" ▽⊸(¬∊"\\r\\n") ◇∘⊡0_1 regex "(?m)^>THREE[^\\n]*\\n([^>]*)" Input
AddCount ← insert ⊃(∘|+1⬚0get|⋅∘)
Count ← ∧AddCount ⊙[] ◫⊙Sequence
Maps ← ≡(□Count) [1 2 3 4 6 12 18]
Fixed ← /◇⊂ {⊃(↘¯3|⋅"."|↙¯3)} ⬚@0↙¯⊸(↥4⧻) °⋕ ⁅×1e3
Entry ← &p $"_ _" ⊃(◇∘⊡1)(Fixed ×100÷⊃(⋅∘)(¯◇∘⊡0))
Frequencies ← (
  ⊙⊸/+ °map
  ⊃(¯⋅⋅∘|∘|⋅∘)
  ≡⌟Entry ⍆≡{⊙∘}
  &p ""
)
Frequencies ◇∘⊡0 Maps
Frequencies ◇∘⊡1 Maps
Queries ← {"GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"}
≡(&p $"_\\t_" ⊃(⬚0get ∩(◇∘))(◇∘)) Queries ↘2 Maps
'''
save('knucleotide','compact',kn);save('knucleotide','fast',kn)
