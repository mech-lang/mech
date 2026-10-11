N     ← ⋕◇∘⊡1 &args
Alu   ← "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
Iub   ← \+[0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02]
Human ← \+[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008]
Wrap  ← ≡◇&p ⊕□⊸(⌊÷60⇡⧻)
~G {Seed Left Letters Cdf}
~Chunk {Draws State}
Size  ← ↧60 G.Left
Next  ← ◿139968 +29573×3877
Draw  ← ⍥(⟜∘Next) ⊃Size G.Seed
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
