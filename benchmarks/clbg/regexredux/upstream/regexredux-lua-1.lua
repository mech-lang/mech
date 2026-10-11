-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jeremy Zerfas.
--
-- WARNING: I normally do my programming in other languages. This may not be an
-- optimal Lua program. Please contribute a better program if you can make a
-- better program.

-- We'll be using the PCRE2 flavor of Lrexlib for our regular expression needs
-- instead of using Lua's built in pattern matching engine. Lua's pattern
-- matching engine is a bit basic and lacks some functionality of other regular
-- expression engines, in particular it doesn't support alternation (e.g. you
-- can't use a pattern like "cat|dog" to search for "cat" or "dog"). It is
-- possible to work around this issue by splitting up the regular expressions
-- into multiple smaller patterns but this could possibly be considered as using
-- a different algorithm and it can result in a faster performing program.
-- Additionally PCRE2 is significantly faster than Lua's pattern matching engine
-- anyway.
Lrexlib_PCRE2=require("rex_pcre2")


-- Read in input from stdin and also get the input_Length.
input=io.read("*all")
input_Length=#input


-- Find all sequence descriptions and new lines in input, replace them with
-- empty strings, and store the result in the sequences string.
regex=Lrexlib_PCRE2.new(">.*\\n|\\n")
regex:jit_compile()
sequences=Lrexlib_PCRE2.gsub(input, regex, "")
sequences_Length=#sequences


-- Iterate through all the count patterns and output the results for each one.
for _, pattern in ipairs({
    "agggtaaa|tttaccct",
    "[cgt]gggtaaa|tttaccc[acg]",
    "a[act]ggtaaa|tttacc[agt]t",
    "ag[act]gtaaa|tttac[agt]ct",
    "agg[act]taaa|ttta[agt]cct",
    "aggg[acg]aaa|ttt[cgt]ccct",
    "agggt[cgt]aa|tt[acg]accct",
    "agggta[cgt]a|t[acg]taccct",
    "agggtaa[cgt]|[acg]ttaccct",
  }) do
	regex=Lrexlib_PCRE2.new(pattern)
	regex:jit_compile()
	io.write(string.format("%s %d\n", pattern, Lrexlib_PCRE2.count(sequences,
	  regex)))
end


-- Copy sequences to postreplace_Sequences and then iterate through all the
-- replacement pairs applying all the replacements to postreplace_Sequences.
postreplace_Sequences=sequences
for _, pattern_Replacement_Pair in ipairs({
    {"tHa[Nt]", "<4>"},
    {"aND|caN|Ha[DS]|WaS", "<3>"},
    {"a[NSt]|BY", "<2>"},
    {"<[^>]*>", "|"},
    {"\\|[^|][^|]*\\|", "-"},
  }) do
	regex=Lrexlib_PCRE2.new(pattern_Replacement_Pair[1])
	regex:jit_compile()
	postreplace_Sequences=Lrexlib_PCRE2.gsub(postreplace_Sequences, regex,
	  pattern_Replacement_Pair[2])
end


-- Print the size of the original input, the size of the input without the
-- sequence descriptions & new lines, and the size after having made all the
-- replacements.
io.write(string.format("\n%d\n%d\n%d\n", input_Length, sequences_Length,
  #postreplace_Sequences))
