app [main!] {
    pf: platform "https://github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU.tar.br",
}
import pf.Stdout
import pf.Stdin
import pf.Arg exposing [Arg]
complement = |byte|
    when byte is
        65 | 97 -> 84u8
        84 | 116 | 85 | 117 -> 65u8
        67 | 99 -> 71u8
        71 | 103 -> 67u8
        77 | 109 -> 75u8
        82 | 114 -> 89u8
        87 | 119 -> 87u8
        83 | 115 -> 83u8
        89 | 121 -> 82u8
        75 | 107 -> 77u8
        86 | 118 -> 66u8
        72 | 104 -> 68u8
        68 | 100 -> 72u8
        66 | 98 -> 86u8
        _ -> 78u8
wrapped! = |bytes, offset, buffer|
    if offset >= List.len(bytes) then Stdout.write_bytes!(buffer) else
        take = Num.min(60,List.len(bytes)-offset)
        line_bytes = List.sublist(bytes,{start:offset,len:take})
        next = List.append(List.concat(buffer,line_bytes),10u8)
        if List.len(next) >= 65536 then
            Stdout.write_bytes!(next)?
            wrapped!(bytes,offset+take,List.with_capacity(65600))
        else wrapped!(bytes,offset+take,next)
record! = |record|
    end = List.find_first_index(record,|b| b == 10) ?? List.len(record)
    header = List.prepend(List.take_first(record,end+1),62u8)
    Stdout.write_bytes!(header)?
    bases = List.keep_if(List.drop_first(record,end+1),|b| b != 10 && b != 13)
    sequence = List.map(List.reverse(bases),complement)
    wrapped!(sequence,0,List.with_capacity(65600))
main! : List Arg => Result {} _
main! = |_args|
    raw = Stdin.read_to_end!({})?
    records = List.drop_first(List.split_on(raw,62u8),1)
    List.for_each_try!(records,record!)
