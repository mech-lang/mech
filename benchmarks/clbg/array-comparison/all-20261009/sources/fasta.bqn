# CLBG LCG, inverse cumulative probabilities, and 60-column FASTA output.
n ← •ParseFloat⊑•args
seed ← 42
alu ← "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
Wrap ← {
  full ← 60×⌊(≠𝕩)÷60
  rows ← (full÷60)‿60⥊full↑𝕩
  •term.OutRaw ⥊{𝕩∾@+10}˘rows
  {0<≠𝕩?•Out 𝕩;@} full↓𝕩
  @
}
Rand ← {139968|29573+3877×𝕨}
Random ← {alphabet‿probabilities‿size:
  r ← seed Rand` size⥊0
  seed ↩ ¯1⊑r
  cumulative ← +`probabilities
  indices ← +´˘⍉ cumulative ≤⌜ r÷139968
  Wrap indices⊏alphabet
}
•Out ">ONE Homo sapiens alu"
Wrap (2×n)⥊alu
•Out ">TWO IUB ambiguity codes"
Random ⟨"acgtBDHKMNRSVWY",0.27‿0.12‿0.12‿0.27∾11⥊0.02,3×n⟩
•Out ">THREE Homo sapiens frequency"
Random ⟨"acgt",0.3029549426680‿0.1979883004921‿0.1975473066391‿0.3015094502008,5×n⟩
