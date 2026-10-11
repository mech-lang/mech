std ← •Import "../stdlib/standard.bqn"
# Read lines; count every overlapping key in a growing built-in hash table.
lines ← •FLines "/dev/stdin"
start ← 1+⊑/">THREE"⊸≡¨6↑¨lines
tail ← start↓lines
end ← ⊑(≠tail)∾˜/'>'=⊑¨tail
dna ← 4|"ACGTacgt"⊐∾end↑tail
Frame ← {𝕊 width‿frame‿piece‿wanted:
 total ← ⌊((≠dna)-frame)÷width
 first ← ⌊total×piece÷8
 last ← ⌊total×(piece+1)÷8
 rows ← last-first
 data ← (width×rows)↑(frame+width×first)↓dna
 keys ← +´˘(rows‿width⥊data)×rows‿width⥊4⋆⌽↕width
 h ← ⟨⟩ •HashMap ⟨⟩
 0 {𝕨 h.Set 1+0 h.Get 𝕨 ⋄ 𝕩}´ keys
 {𝕊 unused: width<3? {0 h.Get 𝕩}¨↕4⋆width; 0 h.Get wanted} @
}
Count ← {𝕊 width‿wanted:
 jobs ← ∾{𝕊 frame: {width‿frame‿𝕩‿wanted}¨↕8}¨↕width
 +´Frame std._Parallel jobs
}
Decode ← {"ACGT"⊏˜4|⌊𝕩÷4⋆⌽↕𝕨}
Fixed ← {s←•Repr⌊0.5+1e3×𝕩 ⋄ ¯3(↓∾'.'∾↑) (((4⌈≠s)-(≠s))⥊'0')∾s}
Frequency ← {𝕊 width:
 counts ← Count width‿0
 keys ← /0<counts
 ordered ← keys⊏˜⍒keys⊏counts
 {•Out (width Decode 𝕩)∾" "∾Fixed 100×(𝕩⊑counts)÷+´counts}¨ordered
 •Out ""
}
Frequency¨ 1‿2
Query ← {𝕊 s:
 key ← +´(4|"ACGTacgt"⊐s)×4⋆⌽↕≠s
 •Out (•Repr Count (≠s)‿key)∾(@+9)∾s
}
Query¨ ⟨"GGT","GGTA","GGTATT","GGTATTTTAATT","GGTATTTTAATTTATAGT"⟩
