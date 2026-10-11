Iub   ← \+[0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02]
Human ← \+[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008]
~G {Seed Left Letters Cdf}
~Chunk {Draws State}
Chars        ← ⊏⊃(/+⊞≥⊃(G.Cdf Chunk.State)(÷139968 Chunk.Draws))(G.Letters Chunk.State)
&w (Chars Chunk (⇡139968) (G 0 0 "acgtBDHKMNRSVWY" Iub)) 1
