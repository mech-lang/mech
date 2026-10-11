# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Written by Jens Adam
# using code and ideas from
# -- David Campbell, Jarrett Revels, Kristoffer Carlsson,
# -- Alex Arslan, Olof Salberger, Adam Beckmeyer
# Optimized by Adam Beckmeyer

const INIO = stdin
const OUTIO = stdout

const COMPS = Vector{UInt8}(undef, 256)
for (i, j) in zip("AaCcGgTtUuMmRrWwSsYyKkVvHhDdBbNn",
                  "TTGGCCAAAAKKYYWWSSRRMMBBDDHHVVNN")
    @inbounds COMPS[i % UInt8] = j % UInt8
end

function print_sequence(v)
    l = 1
    # Don't count the last newline
    r = length(v) - 1

    @inbounds while true
        vl, vr = v[l], v[r]
        if vl == '\n' % UInt8
            l += 1
            vl = v[l]
        end
        if vr == '\n' % UInt8
            r -= 1
            vr = v[r]
        end
        l > r && break
        v[l], v[r] = COMPS[vr], COMPS[vl]
        l += 1
        r -= 1
    end
    write(OUTIO, v)
end

function perf_revcomp()
    seek(INIO, 1)
    while !eof(INIO)
        # Seek back 1 since previous loop put position on '>'
        seek(INIO, position(INIO) - 1)
        # Write header line
        write(OUTIO, readline(INIO; keep=true))
        # Perform revcomp on sequence.
        # Julia buffers all IOStreams by no more than 131072
        # bytes. While readuntil does not fulfill the letter of the
        # law in "read line-by-line", doing so is impossible since
        # regardless of user machinations, Julia always reads 128 kb
        # at a time from io; there is no way to disable this
        # buffering. Other implementations (e.g. Matt Brubeck's Rust
        # 2) also use this pattern. The only difference is that
        # Julia's buffer-size is 128 kb while Rust's is only 8 kb.
        print_sequence(readuntil(INIO, '>' % UInt8; keep=false))
    end
end

isinteractive() || perf_revcomp()
