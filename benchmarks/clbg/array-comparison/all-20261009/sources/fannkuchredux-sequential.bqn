# Enumerate all permutations in rotation order, flipping every permutation.
n ← •ParseFloat⊑•args
p ← ↕n
counts ← n⥊0
index ← 0
checksum ← 0
maximum ← 0
Flip ← {size←1+⊑𝕩 ⋄ (⌽size↑𝕩)∾size↓𝕩}
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
Step •_while_ ⊢ 1
•Out •Repr checksum
•Out "Pfannkuchen("∾(•Repr n)∾") = "∾•Repr maximum
