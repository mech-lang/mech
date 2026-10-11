"""General pure-Roc integer and byte-regex helpers, counted in application LOC."""
BIG='''Big : {negative : Bool, digits : List U64}

base = 4294967296u64
trim_digits = |digits|
    if List.is_empty(digits) then digits else
        if get(digits,List.len(digits)-1) == 0 then trim_digits(List.drop_last(digits,1)) else digits
big : Bool, List U64 -> Big
big = |negative, digits|
    normalized = trim_digits(digits)
    {negative:negative && !List.is_empty(normalized),digits:normalized}
natural = |value|
    if value == 0 then big(Bool.false,[]) else
        if value < base then big(Bool.false,[value]) else big(Bool.false,[value%base,value//base])
compare_digits = |a,b,remaining|
    if remaining == 0 then EQ else
        index = remaining-1
        order = Num.compare(get(a,index),get(b,index))
        if order == EQ then compare_digits(a,b,index) else order
compare_abs = |a,b|
    order = Num.compare(List.len(a),List.len(b))
    if order == EQ then compare_digits(a,b,List.len(a)) else order
add_digits = |a,b,index,carry,output|
    length = Num.max(List.len(a),List.len(b))
    if index >= length then
        if carry == 0 then output else List.append(output,carry)
    else
        av = List.get(a,index) ?? 0
        bv = List.get(b,index) ?? 0
        sum = av+bv+carry
        add_digits(a,b,index+1,sum//base,List.append(output,sum%base))
add_abs = |a,b| add_digits(a,b,0,0,List.with_capacity(Num.max(List.len(a),List.len(b))+1))
subtract_digits = |a,b,index,borrow,output|
    if index >= List.len(a) then trim_digits(output) else
        av = get(a,index)
        bv = (List.get(b,index) ?? 0)+borrow
        digit = if av < bv then av+base-bv else av-bv
        next_borrow = if av < bv then 1u64 else 0u64
        subtract_digits(a,b,index+1,next_borrow,List.append(output,digit))
subtract_abs = |a,b| subtract_digits(a,b,0,0,List.with_capacity(List.len(a)))
big_add : Big, Big -> Big
big_add = |a,b|
    if a.negative == b.negative then big(a.negative,add_abs(a.digits,b.digits)) else
        when compare_abs(a.digits,b.digits) is
            LT -> big(b.negative,subtract_abs(b.digits,a.digits))
            _ -> big(a.negative,subtract_abs(a.digits,b.digits))
big_sub = |a,b| big_add(a,big(!b.negative,b.digits))
multiply_digits = |digits,factor,index,carry,output|
    if index >= List.len(digits) then
        if carry == 0 then output else
            multiply_digits(digits,factor,index,carry//Num.to_u128(base),List.append(output,Num.to_u64(carry%Num.to_u128(base))))
    else
        product = Num.to_u128(get(digits,index))*Num.to_u128(factor)+carry
        digit = Num.to_u64(product%Num.to_u128(base))
        multiply_digits(digits,factor,index+1,product//Num.to_u128(base),List.append(output,digit))
big_mul = |a,factor| big(a.negative,multiply_digits(a.digits,factor,0,0u128,List.with_capacity(List.len(a.digits)+2)))
bit_length = |digits|
    if List.is_empty(digits) then 0u64 else
        top = get(digits,List.len(digits)-1)
        (List.len(digits)-1)*32+64-Num.to_u64(Num.count_leading_zero_bits(top))
shift_bits = |digits,bits|
    scale = Num.shift_left_by(1u64,Num.to_u8(bits%32))
    multiplied = big_mul(big(Bool.false,digits),scale)
    List.concat(List.repeat(0u64,bits//32),multiplied.digits)
power_bit = |bits| List.append(List.repeat(0u64,bits//32),Num.shift_left_by(1u64,Num.to_u8(bits%32)))
divide_abs = |remainder,divisor,quotient|
    if compare_abs(remainder,divisor) == LT then (quotient,remainder) else
        difference = bit_length(remainder)-bit_length(divisor)
        candidate = shift_bits(divisor,difference)
        bits = if compare_abs(candidate,remainder) == GT then difference-1 else difference
        scaled = if bits == difference then candidate else shift_bits(divisor,bits)
        next = subtract_abs(remainder,scaled)
        updated = add_abs(quotient,power_bit(bits))
        divide_abs(next,divisor,updated)
big_floor : Big, Big -> Big
big_floor = |a,b|
    if List.is_empty(b.digits) then crash("division by zero") else
        (quotient,remainder) = divide_abs(a.digits,b.digits,[])
        negative = a.negative != b.negative
        rounded = if negative && !List.is_empty(remainder) then add_abs(quotient,[1u64]) else quotient
        big(negative,rounded)
'''

PI='''extract = |factor,q,r,t| big_floor(big_add(big_mul(q,factor),r),t)
spigot = |n,produced,k,q,r,t,pending,rows|
    if produced == n then Str.join_with(rows,"") else
        first = extract(3,q,r,t)
        second = extract(4,q,r,t)
        if first == second then
            digit = List.first(first.digits) ?? 0
            count = produced+1
            line = Str.concat(pending,Num.to_str(digit))
            complete = count%10 == 0 || count == n
            next_rows = if complete then
                padding = Str.repeat(" ",10-Str.count_utf8_bytes(line))
                List.append(rows,"${line}${padding}\\t:${Num.to_str(count)}\\n")
            else rows
            next_pending = if complete then "" else line
            next_r = big_mul(big_sub(r,big_mul(t,digit)),10)
            next_q = big_mul(q,10)
            spigot(n,count,k,next_q,next_r,t,next_pending,next_rows)
        else
            next_k = k+1
            multiplier = 2*next_k+1
            next_r = big_mul(big_add(r,big_mul(q,2)),multiplier)
            next_q = big_mul(q,next_k)
            next_t = big_mul(t,multiplier)
            spigot(n,produced,next_k,next_q,next_r,next_t,pending,rows)
main! : List Arg => Result {} _
main! = |args|
    Stdout.write!(spigot(argument(args),0,0,natural(1),natural(0),natural(1),"",[]))
'''

REGEX='''Pattern : [Literal U8, Dot, Class Bool (List U8), Sequence (List Pattern), Alternate (List Pattern), Repeat Pattern U64 U64, Begin, End]

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
find_next = |pattern,input,position,insensitive|
    if position > List.len(input) then NotFound else
        possible = if position == List.len(input) then nullable(pattern) else can_start(pattern,get(input,position),insensitive)
        ends = if possible then match_pattern(pattern,input,position,insensitive) else []
        when List.first(ends) is
            Ok(last) -> Found(position,last)
            Err(_) -> if position == List.len(input) then NotFound else find_next(pattern,input,position+1,insensitive)
count_matches = |pattern,input,position,insensitive,total|
    when find_next(pattern,input,position,insensitive) is
        NotFound -> total
        Found(first,last) -> count_matches(pattern,input,if first == last then last+1 else last,insensitive,total+1)
replace_from = |pattern,input,replacement,position,output|
    if position > List.len(input) then output else
        when find_next(pattern,input,position,Bool.false) is
            NotFound -> List.concat(output,List.sublist(input,{start:position,len:List.len(input)-position}))
            Found(first,last) ->
                before = List.sublist(input,{start:position,len:first-position})
                joined = List.concat(List.concat(output,before),replacement)
                if first == last && last < List.len(input) then
                    replace_from(pattern,input,replacement,last+1,List.append(joined,get(input,last)))
                else replace_from(pattern,input,replacement,if first == last then last+1 else last,joined)
replace_regex = |pattern,input,replacement| replace_from(pattern,input,Str.to_utf8(replacement),0,List.with_capacity(List.len(input)))
'''

REGEX_BENCH='''main! : List Arg => Result {} _
main! = |_args|
    raw = Stdin.read_to_end!({})?
    cleaned = replace_regex(compile(">.*\\n|\\n"),raw,"")
    patterns = [
        "agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct",
    ]
    rows = List.map(patterns,|text|
        count = count_matches(compile(text),cleaned,0,Bool.true,0u64)
        "${text} ${Num.to_str(count)}\\n",
    )
    replacements = [("tHa[Nt]","<4>"),("aND|caN|Ha[DS]|WaS","<3>"),("a[NSt]|BY","<2>"),("<[^>]*>","|"),("\\\\|[^|][^|]*\\\\|","-")]
    replaced = List.walk(replacements,cleaned,|bytes,pair|
        (pattern,replacement) = pair
        replace_regex(compile(pattern),bytes,replacement),
    )
    output = Str.join_with(rows,"")
    Stdout.write!("${output}\\n${Num.to_str(List.len(raw))}\\n${Num.to_str(List.len(cleaned))}\\n${Num.to_str(List.len(replaced))}\\n")
'''
