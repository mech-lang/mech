app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Arg exposing [Arg]
argument : List Arg -> U64
argument = |args|
    arg = (List.get(args, 1) |> Result.map_ok(Arg.display)) ?? "0"
    Str.to_u64(arg) ?? 0
get = |items, index|
    when List.get(items,index) is
        Ok(value) -> value
        Err(_) -> crash("index out of bounds")
Big : {negative : Bool, digits : List U64}

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
extract = |factor,q,r,t| big_floor(big_add(big_mul(q,factor),r),t)
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
                List.append(rows,"${line}${padding}\t:${Num.to_str(count)}\n")
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
