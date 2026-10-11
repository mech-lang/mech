i1h←0.5×i1←1+i←↕n←•ParseFloat⊑•args
b ← ÷ (i×0.5) {i1 + (i+𝕩) × i1h+𝕨}˘ i
a ← ⍉b ⋄ a‿b<˘¨↩
S ← •math.Sum
BF←{(S 𝕩⊸×)¨ b}
AF←{(S 𝕩⊸×)¨ a}
v ← BF∘AF⍟18 BF S¨ a
•Out {¯9(↓∾'.'∾↑)•Repr⌊0.5+1e9×𝕩} √ (v×BF AF v) ÷○S ×˜v
