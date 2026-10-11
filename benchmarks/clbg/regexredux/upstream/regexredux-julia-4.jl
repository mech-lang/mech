# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Daniel Jones
# fixed by David Campbell
# modified by Jarret Revels, Alex Arslan, Yichao Yu
# made multi-threaded by Adam Beckmeyer

# Julia's replace function doesn't internally use pcre2_substitute
# which is more efficient than the string-building Julia currently
# uses. This gives about a 10% speedup compared to using Base.replace.
function rereplace!(s, slen, pat::Pair{Regex,String}, dest)
    re, rep = pat
    dest_len = Ref{Csize_t}(sizeof(dest))
    Base.compile(re)
    ccall((:pcre2_substitute_8, Base.PCRE.PCRE_LIB), Cint,
          (Ptr{Cvoid}, Ptr{UInt8}, Csize_t, Csize_t, Cuint, Ptr{Cvoid}, Ptr{Cvoid},
           Ptr{UInt8}, Csize_t, Ptr{UInt8}, Ref{Csize_t}), re.regex, s, slen,
          0, re.match_options | Base.PCRE.SUBSTITUTE_GLOBAL,
          C_NULL, C_NULL, rep, sizeof(rep), dest, dest_len)
    dest_len[]
end

const variants = (
      r"agggtaaa|tttaccct",
      r"[cgt]gggtaaa|tttaccc[acg]",
      r"a[act]ggtaaa|tttacc[agt]t",
      r"ag[act]gtaaa|tttac[agt]ct",
      r"agg[act]taaa|ttta[agt]cct",
      r"aggg[acg]aaa|ttt[cgt]ccct",
      r"agggt[cgt]aa|tt[acg]accct",
      r"agggta[cgt]a|t[acg]taccct",
      r"agggtaa[cgt]|[acg]ttaccct"
)

const subs = (
    r"tHa[Nt]"            => "<4>",
    r"aND|caN|Ha[DS]|WaS" => "<3>",
    r"a[NSt]|BY"          => "<2>",
    r"<[^>]*>"            => "|",
    r"\|[^|][^|]*\|"      => "-"
)

function main(io, out)
    # Read in the file as a string and get length
    seq = read(io, String)
    l1 = sizeof(seq)

    # Remove descriptions and linefeed characters while copying seq to seq2
    seq2 = Vector{UInt8}(undef, l1)
    l2 = rereplace!(seq, sizeof(seq), r">.*\n|\n" => "", seq2)
    resize!(seq2, l2)

    # Turning seq2 back into a string will cause it to be truncated to
    # zero, so we first need to make the working copies that will be
    # needed for the replace task. These must be slightly larger than
    # seq2 since some replacements can cause the string to grow.
    repseq = similar(seq2, trunc(Int, 1.1l2))
    outseq = similar(repseq)
    copyto!(repseq, seq2)

    # First we start the task to find the length of the string with
    # all replacements since it takes the longest.
    l3 = l2 % Csize_t
    replace_task = Threads.@spawn for sub in subs
        # This loop moves back and forth, first doing a replacement
        # while copying repseq to outseq and then another replacement
        # while copying outseq back to repseq.
        l3 = rereplace!(repseq, l3, sub, outseq)
        repseq, outseq = outseq, repseq
    end

    # Then we count the occurrences of each variant within
    # seq. Although this could be done multi-threaded with a counter
    # per thread, there's no benefit to doing so since replace_task
    # takes longer than all variant counts in serial.
    seq = String(seq2)
    for v in variants
        # Using count(variants[i], seq) isn't as fast. Not sure why.
        println(out, v.pattern, ' ', length(collect(eachmatch(v, seq))))
    end

    println(out, '\n', l1)
    println(out, l2)
    # Wait for the replacing task to complete before printing sequence lengths.
    wait(replace_task)
    println(out, l3)
end

isinteractive() || main(stdin, stdout)
