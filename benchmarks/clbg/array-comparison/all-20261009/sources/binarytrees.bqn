std ← •Import "../stdlib/standard.bqn"
# Every node is an allocated pair; checks traverse the constructed trees.
n ← 6⌈•ParseFloat⊑•args
Node ← {⟨𝕨,𝕩⟩}
Tree ← {𝕩=0? (𝕩↑⟨⟩) Node 𝕩↑⟨⟩; (𝕊𝕩-1) Node 𝕊𝕩-1}
Check ← {0=≠⊑𝕩?1;1++´𝕊¨𝕩}
tab ← @+9
•Out "stretch tree of depth "∾(•Repr n+1)∾tab∾" check: "∾•Repr Check Tree n+1
long ← Tree n
Compute ← {
  iterations ← 2⋆n-𝕩-4
  total ← +´{Check Tree 𝕩}¨iterations⥊𝕩
  iterations‿𝕩‿total
}
results ← Compute std._Parallel 4+2×↕1+⌊(n-4)÷2
{𝕊 iterations‿depth‿total: •Out (•Repr iterations)∾tab∾" trees of depth "∾(•Repr depth)∾tab∾" check: "∾•Repr total}¨results
•Out "long lived tree of depth "∾(•Repr n)∾tab∾" check: "∾•Repr Check long
