app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Stdin
import pf.Arg exposing [Arg]
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
Pattern : [Literal U8, Dot, Class Bool (List U8), Sequence (List Pattern), Alternate (List Pattern), Repeat Pattern U64 U64, Begin, End]

escaped = |byte|
    when byte is
        110 -> 10u8
        114 -> 13u8
        116 -> 9u8
        _ -> byte
class_chars = |tokens,index,chars|
    if index >= List.len(tokens) then crash("unterminated character class") else
        value = get(tokens,index)
        if value == 93 then (chars,index+1) else
            (byte,next) = if value == 92 then (escaped(get(tokens,index+1)),index+2) else (value,index+1)
            if next+1 < List.len(tokens) && get(tokens,next) == 45 && get(tokens,next+1) != 93 then
                last = get(tokens,next+1)
                expanded = List.range({start:At(byte),end:At(last)})
                class_chars(tokens,next+2,List.concat(chars,expanded))
            else class_chars(tokens,next,List.append(chars,byte))
parse_alt : List U8, U64 -> (Pattern,U64)
parse_alt = |tokens,index|
    (sequence,next) = parse_sequence(tokens,index,[])
    if next < List.len(tokens) && get(tokens,next) == 124 then
        (rest,after) = parse_alt(tokens,next+1)
        (Alternate([sequence,rest]),after)
    else (sequence,next)
parse_sequence : List U8, U64, List Pattern -> (Pattern,U64)
parse_sequence = |tokens,index,parts|
    if index >= List.len(tokens) then (Sequence(parts),index) else
        token = get(tokens,index)
        if token == 124 || token == 41 then (Sequence(parts),index) else
            (atom,next) = parse_atom(tokens,index)
            (piece,after) = if next < List.len(tokens) then
                when get(tokens,next) is
                    42 -> (Repeat(atom,0,Num.max_u64),next+1)
                    43 -> (Repeat(atom,1,Num.max_u64),next+1)
                    63 -> (Repeat(atom,0,1),next+1)
                    _ -> (atom,next)
            else (atom,next)
            parse_sequence(tokens,after,List.append(parts,piece))
parse_atom : List U8, U64 -> (Pattern,U64)
parse_atom = |tokens,index|
    token = get(tokens,index)
    when token is
        40 ->
            (group,next) = parse_alt(tokens,index+1)
            if next >= List.len(tokens) || get(tokens,next) != 41 then crash("unterminated group") else (group,next+1)
        91 ->
            negated = index+1 < List.len(tokens) && get(tokens,index+1) == 94
            start = if negated then index+2 else index+1
            (chars,next) = class_chars(tokens,start,[])
            (Class(negated,chars),next)
        92 -> (Literal(escaped(get(tokens,index+1))),index+2)
        46 -> (Dot,index+1)
        94 -> (Begin,index+1)
        36 -> (End,index+1)
        _ -> (Literal(token),index+1)
compile = |text|
    tokens = Str.to_utf8(text)
    (pattern,index) = parse_alt(tokens,0)
    if index != List.len(tokens) then crash("unexpected regex token") else pattern
fold_ascii = |byte| if byte >= 65 && byte <= 90 then byte+32 else byte
same_byte = |a,b,insensitive| if insensitive then fold_ascii(a) == fold_ascii(b) else a == b
atom_matches = |pattern,byte,insensitive|
    when pattern is
        Literal(value) -> same_byte(value,byte,insensitive)
        Dot -> byte != 10
        Class(negated,chars) -> negated != List.any(chars,|value| same_byte(value,byte,insensitive))
        _ -> Bool.false
nullable = |pattern|
    when pattern is
        Sequence(parts) -> List.all(parts,nullable)
        Alternate(parts) -> List.any(parts,nullable)
        Repeat(piece,minimum,_) -> minimum == 0 || nullable(piece)
        Begin | End -> Bool.true
        _ -> Bool.false
sequence_starts = |parts,byte,insensitive|
    when parts is
        [] -> Bool.true
        [head,.. as rest] -> can_start(head,byte,insensitive) || (nullable(head) && sequence_starts(rest,byte,insensitive))
can_start = |pattern,byte,insensitive|
    when pattern is
        Sequence(parts) -> sequence_starts(parts,byte,insensitive)
        Alternate(parts) -> List.any(parts,|p| can_start(p,byte,insensitive))
        Repeat(piece,minimum,_) -> minimum == 0 || can_start(piece,byte,insensitive)
        Begin | End -> Bool.true
        _ -> atom_matches(pattern,byte,insensitive)
match_sequence : List Pattern, List U8, U64, Bool -> List U64
match_sequence = |parts,input,position,insensitive|
    when parts is
        [] -> [position]
        [piece,.. as rest] ->
            endings = match_pattern(piece,input,position,insensitive)
            List.join_map(endings,|next| match_sequence(rest,input,next,insensitive))
match_repeat : Pattern, List U8, U64, Bool, U64, U64, U64 -> List U64
match_repeat = |piece,input,position,insensitive,count,minimum,maximum|
    current = if count >= minimum then [position] else []
    if count == maximum then current else
        endings = List.keep_if(match_pattern(piece,input,position,insensitive),|next| next > position)
        advanced = List.join_map(endings,|next| match_repeat(piece,input,next,insensitive,count+1,minimum,maximum))
        List.concat(advanced,current)
match_pattern : Pattern, List U8, U64, Bool -> List U64
match_pattern = |pattern,input,position,insensitive|
    when pattern is
        Sequence(parts) -> match_sequence(parts,input,position,insensitive)
        Alternate(parts) -> List.join_map(parts,|piece| match_pattern(piece,input,position,insensitive))
        Repeat(piece,minimum,maximum) -> match_repeat(piece,input,position,insensitive,0,minimum,maximum)
        Begin -> if position == 0 then [position] else []
        End -> if position == List.len(input) then [position] else []
        _ ->
            if position < List.len(input) && atom_matches(pattern,get(input,position),insensitive) then [position+1] else []
prepare_regex = |pattern,insensitive|
    starts = List.map(List.range({start:At(0u64),end:Before(256u64)}),|byte| can_start(pattern,Num.to_u8(byte),insensitive))
    {pattern:pattern,starts:starts,insensitive:insensitive,at_end:nullable(pattern)}
find_compiled = |compiled,input,position|
    if position > List.len(input) then NotFound else
        possible = if position == List.len(input) then compiled.at_end else (List.get(compiled.starts,Num.to_u64(get(input,position))) ?? Bool.false)
        ends = if possible then match_pattern(compiled.pattern,input,position,compiled.insensitive) else []
        when List.first(ends) is
            Ok(last) -> Found(position,last)
            Err(_) -> if position == List.len(input) then NotFound else find_compiled(compiled,input,position+1)
count_compiled = |compiled,input,position,total|
    when find_compiled(compiled,input,position) is
        NotFound -> total
        Found(first,last) -> count_compiled(compiled,input,if first == last then last+1 else last,total+1)
count_matches = |pattern,input,position,insensitive,total| count_compiled(prepare_regex(pattern,insensitive),input,position,total)
replace_from = |compiled,input,replacement,position,output|
    if position > List.len(input) then output else
        when find_compiled(compiled,input,position) is
            NotFound -> List.concat(output,List.sublist(input,{start:position,len:List.len(input)-position}))
            Found(first,last) ->
                before = List.sublist(input,{start:position,len:first-position})
                joined = List.concat(List.concat(output,before),replacement)
                if first == last && last < List.len(input) then
                    replace_from(compiled,input,replacement,last+1,List.append(joined,get(input,last)))
                else replace_from(compiled,input,replacement,if first == last then last+1 else last,joined)
replace_regex = |pattern,input,replacement| replace_from(prepare_regex(pattern,Bool.false),input,Str.to_utf8(replacement),0,List.with_capacity(List.len(input)))

expect count_matches(compile("ab+c"),Str.to_utf8("abbbbcabcabcc"),0,Bool.false,0u64) == 3
expect replace_regex(compile("ab+c"),Str.to_utf8("abbbbcabcabcc"),"X") == Str.to_utf8("XXXc")
expect count_matches(compile("(ab|cd)+"),Str.to_utf8("xxababcdzcd"),0,Bool.false,0u64) == 2
expect replace_regex(compile("(ab|cd)+"),Str.to_utf8("xxababcdzcd"),"X") == Str.to_utf8("xxXzX")
expect count_matches(compile("a[0-9]?b"),Str.to_utf8("a1b ab axb a11b"),0,Bool.false,0u64) == 2
expect replace_regex(compile("a[0-9]?b"),Str.to_utf8("a1b ab axb a11b"),"X") == Str.to_utf8("X X axb a11b")
expect count_matches(compile("^[a-z]+$"),Str.to_utf8("HELLO"),0,Bool.true,0u64) == 1
expect count_matches(compile("a.*b"),Str.to_utf8("a123b45b\na0b"),0,Bool.false,0u64) == 2
expect replace_regex(compile("a.*b"),Str.to_utf8("a123b45b\na0b"),"X") == Str.to_utf8("X\nX")
expect count_matches(compile("<[^>]*>"),Str.to_utf8("one<two><three>four"),0,Bool.false,0u64) == 2
expect replace_regex(compile("<[^>]*>"),Str.to_utf8("one<two><three>four"),"X") == Str.to_utf8("oneXXfour")
expect count_matches(compile("a?"),Str.to_utf8("baab"),0,Bool.false,0u64) == 5
expect replace_regex(compile("a?"),Str.to_utf8("baab"),"X") == Str.to_utf8("XbXXXbX")
expect count_matches(compile("agggtaaa|tttaccct"),Str.to_utf8("AGGGTAAA tttaccct"),0,Bool.true,0u64) == 2
main! : List Arg => Result {} _
main! = |_args| Stdout.line!("general optimized regex tests")
