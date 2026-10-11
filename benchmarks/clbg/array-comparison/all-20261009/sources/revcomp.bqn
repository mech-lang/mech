# Preserve FASTA headers, reverse each complete sequence, and wrap at 60 columns.
lines ← •FLines "/dev/stdin"
starts ← / '>'=⊑¨lines
boundaries ← starts∾≠lines
alphabet ← "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
complement ← "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
Record ← {a‿b:
  •Out a⊑lines
  sequence ← ∾(1+a+↕b-a+1)⊏lines
  result ← ⌽complement⊏˜alphabet⊐sequence
  full ← 60×⌊(≠result)÷60
  rows ← (full÷60)‿60⥊full↑result
  •term.OutRaw ⥊{𝕩∾@+10}˘rows
  {0<≠𝕩?•Out 𝕩;@} full↓result
  @
}
Record˘ 2↕boundaries
