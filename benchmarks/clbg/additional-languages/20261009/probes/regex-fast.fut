-- General byte regular expressions: concatenation, ordered alternation,
-- dot, escapes, classes/ranges/negation, and greedy *, +, ? quantifiers.
-- Capturing groups, anchors, backreferences and Unicode are unsupported.
-- All parsing, searching and substitution execute in Futhark.
type~ regexp = ([][4]u64,[]i64,[]i64,[]i64)
def escaped (c:u8):u8 = if c == 110 then 10 else if c == 114 then 13 else if c == 116 then 9 else c
def literal (pattern:[]u8) (p:i64) =
  if pattern[p] == 92 && p+1 < length pattern then (escaped pattern[p+1],p+2)
  else (pattern[p],p+1)
def mask_set (mask:*[4]u64) (c:u8):*[4]u64 =
  let word = i64.u8 c/64
  let mask[word] = mask[word] | (1u64 << u64.u8 (c%64))
  in mask
def class_mask (pattern:[]u8) (first:i64):([4]u64,i64) =
  let negative = first+1 < length pattern && pattern[first+1] == 94
  let start = first+1+(if negative then 1 else 0)
  let (mask,p) = loop (mask,p) = (replicate 4 0u64,start)
    while p < length pattern && (pattern[p] != 93 || p == start) do
      let (lo,next) = literal pattern p
      let range = next+1 < length pattern && pattern[next] == 45 && pattern[next+1] != 93
      let (hi,next) = if range then literal pattern (next+1) else (lo,next)
      let mask = loop mask = mask for value in i64.u8 lo..<i64.u8 hi+1 do mask_set mask (u8.i64 value)
      in (mask,next)
  in (if negative then map (\x -> x ^ u64.highest) mask else mask,p+1)
def compile (pattern:[]u8):regexp =
  let size = length pattern+1
  let (_p,atoms,repeats,starts,ends,used,branches) =
    loop (p,atoms,repeats,starts,ends,used,branch) =
      (0i64,replicate size (replicate 4 0u64),replicate size 1i64,replicate size 0i64,replicate size 0i64,0i64,0i64)
      while p < length pattern do
        if pattern[p] == 124 then
          let ends[branch] = used
          let starts[branch+1] = used
          in (p+1,atoms,repeats,starts,ends,used,branch+1)
        else
          let (mask,next) = if pattern[p] == 91 then class_mask pattern p
            else if pattern[p] == 46 then
              let mask = replicate 4 u64.highest
              let mask[0] = mask[0] ^ (1u64 << 10)
              in (mask,p+1)
            else let (c,next) = literal pattern p
                 in (mask_set (replicate 4 0u64) c,next)
          let quantifier = if next >= length pattern then 1
            else if pattern[next] == 42 then 0
            else if pattern[next] == 43 then 2
            else if pattern[next] == 63 then 3 else 1
          let atoms[used] = mask
          let repeats[used] = quantifier
          in (next+(if quantifier == 1 then 0 else 1),atoms,repeats,starts,ends,used+1,branch)
  let ends[branches] = used
  in (take used atoms,take used repeats,take (branches+1) starts,take (branches+1) ends)
def accepts (mask:[4]u64) (c:u8):bool =
  ((mask[i64.u8 c/64] >> u64.u8 (c%64))&1) != 0
def match_branch (rx:regexp) (token:i64) (last:i64) (text:[]u8) (position:i64):i64 =
  if token == last then position
  else
    let kind = rx.1[token]
    let minimum = if kind == 0 || kind == 3 then 0 else 1
    let limit = if kind == 1 || kind == 3 then i64.min (position+1) (length text) else length text
    let end = loop end = position while end < limit && accepts rx.0[token] text[end] do end+1
    let (_trial,result) = loop (trial,result) = (end,-1i64)
      while trial >= position+minimum && result < 0 do
        let result = match_branch rx (token+1) last text trial
        in (trial-1,result)
    in result
def find (rx:regexp) (text:[]u8) (start:i64):(i64,i64) =
  let (position,end) = loop (position,end) = (start,-1i64)
    while position <= length text && end < 0 do
      let end = loop result = -1i64 for branch < length rx.2 do
        if result >= 0 then result
        else let first = rx.2[branch]
             let last = rx.3[branch]
             let possible = first == last || rx.1[first] == 0 || rx.1[first] == 3 ||
               (position < length text && accepts rx.0[first] text[position])
             in if possible then match_branch rx first last text position else -1
      in (if end < 0 then position+1 else position,end)
  in (position,end)
def count_general (rx:regexp) (text:[]u8):i64 =
  let (_position,count) = loop (position,count) = (0i64,0i64) while position <= length text do
    let (first,last) = find rx text position
    in if last < 0 then (length text+1,count)
       else (if last == first then last+1 else last,count+1)
  in count
def fixed_match (rx:regexp) (text:[]u8) (position:i64) (width:i64):bool =
  loop found = false for branch < length rx.2 do
    if found then true else
      let first = rx.2[branch]
      let (_i,ok) = loop (i,ok) = (0i64,true) while i < width && ok do
        (i+1,#[inline] accepts rx.0[first+i] text[position+i])
      in ok
def count (rx:regexp) (text:[]u8):i64 =
  let width = rx.3[0]-rx.2[0]
  let fixed = width > 0 && all (==1) rx.1 && all (\i -> rx.3[i]-rx.2[i] == width) (iota (length rx.2))
  in if !fixed then count_general rx text else
    let matches = map (\i -> fixed_match rx text i width) (iota (i64.max 0 (length text-width+1)))
    let (_i,total) = loop (i,total) = (0i64,0i64) while i < length matches do
      if matches[i] then (i+width,total+1) else (i+1,total)
    in total
def replace (rx:regexp) (text:[]u8) (replacement:[]u8):*[]u8 =
  let capacity = length text+(length text+1)*length replacement
  let (_position,out,used) = loop (position,out,used) = (0i64,replicate capacity 0u8,0i64)
    while position <= length text do
      let (first,last) = find rx text position
      let finish = if last < 0 then length text else first
      let out = loop out = out for i in position..<finish do
        let out[used+i-position] = text[i]
        in out
      let used = used+finish-position
      in if last < 0 then (length text+1,out,used)
         else let out = loop out = out for i < length replacement do
                let out[used+i] = replacement[i]
                in out
              let used = used+length replacement
              in if first == last && last < length text then
                   let out[used] = text[last]
                   in (last+1,out,used+1)
                 else (if first == last then last+1 else last,out,used)
  in take used out
