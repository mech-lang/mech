# Eight independent BQN workers; all permutation and flip work is in Work.
std ← •Import "../stdlib/standard.bqn"
n ← •ParseFloat⊑•args
facts ← 1∾×`1+↕n
Flip ← {size←1+⊑𝕩 ⋄ (⌽size↑𝕩)∾size↓𝕩}
Work ← {𝕊 first‿last:
 p ← ↕n
 counts ← n⥊0
 index ← first
 checksum ← 0
 maximum ← 0
 rank ← first
 Init ← {
  d ← ⌊rank÷𝕩⊑facts
  counts ↩ d⌾(𝕩⊸⊑) counts
  rank ↩ (𝕩⊑facts)|rank
  size ← 𝕩+1
  p ↩ (d⌽size↑p)∾size↓p
 }
 Init¨ ⌽1+↕n-1
 Step ← {𝕊 unused:
  flips ← 0
  {flips +↩ 1 ⋄ Flip 𝕩} •_while_ {0≠⊑𝕩} p
  checksum +↩ (1-2×2|index)×flips
  maximum ⌈↩ flips
  r ← 1
  Carry ← {𝕊 unused:
   size ← r+1
   p ↩ (1⌽size↑p)∾size↓p
   {𝕊 unused: r=r⊑counts?
    counts ↩ 0⌾(r⊸⊑) counts
    r +↩ 1
    1;
    counts ↩ (1+r⊑counts)⌾(r⊸⊑) counts
    0
   } @
  }
  Carry •_while_ {𝕩∧r<n} 1
  index +↩ 1
  r<n
 }
 Step •_while_ {𝕊 unused: index<last} @
 checksum‿maximum
}
cuts ← ⌊(↕9)×(¯1⊑facts)÷8
results ← Work std._Parallel <˘2↕cuts
•Out •Repr +´⊑¨results
•Out "Pfannkuchen("∾(•Repr n)∾") = "∾•Repr ⌈´{1⊑𝕩}¨results
