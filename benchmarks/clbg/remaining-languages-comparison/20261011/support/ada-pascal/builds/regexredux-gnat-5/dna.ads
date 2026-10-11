--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Restarted from RegexDNA program, Georg Bauhaus in March 2017
--
--  This version uses the GNAT Spitbol Pattern matching libraries
--  rather than the more commonly used Unix-style regex libraries.

with GNAT.Spitbol.Patterns;     use GNAT.Spitbol.Patterns,
                                    GNAT.Spitbol;
with U;
package DNA is

   subtype Variant_Index is Positive range 1 .. 9;
   Variant_Labels : constant array (Variant_Index) of VString := (
      V ("agggtaaa|tttaccct"),
      V ("[cgt]gggtaaa|tttaccc[acg]"),
      V ("a[act]ggtaaa|tttacc[agt]t"),
      V ("ag[act]gtaaa|tttac[agt]ct"),
      V ("agg[act]taaa|ttta[agt]cct"),
      V ("aggg[acg]aaa|ttt[cgt]ccct"),
      V ("agggt[cgt]aa|tt[acg]accct"),
      V ("agggta[cgt]a|t[acg]taccct"),
      V ("agggtaa[cgt]|[acg]ttaccct"));

   Variant_Patterns : constant array (Variant_Index) of Pattern :=
     ( --  corresponding alternations in SPITBOL notation
       1 => ((BreakX ("a") & "agggtaaa") or
             (BreakX ("t") & "tttaccct") or
             Cancel),
       2 => ((BreakX ("cgt") & Any ("cgt") & "gggtaaa") or
             (BreakX ("t") & "tttaccc" & Any ("acg")) or
             Cancel),
       3 => ((BreakX ("a") & "a" & Any ("act") & "ggtaaa") or
             (BreakX ("t") & "tttacc" & Any ("agt") & "t") or
             Cancel),
       4 => ((BreakX ("a") & "ag" & Any ("act") & "gtaaa") or
             (BreakX ("t") & "tttac" & Any ("agt") & "ct") or
             Cancel),
       5 => ((BreakX ("a") & "agg" & Any ("act") & "taaa") or
             (BreakX ("t") & "ttta" & Any ("agt") & "cct") or
             Cancel),
       6 => ((BreakX ("a") & "aggg" & Any ("acg") & "aaa") or
             (BreakX ("t") & "ttt" & Any ("cgt") & "ccct") or
             Cancel),
       7 => ((BreakX ("a") & "agggt" & Any ("cgt") & "aa") or
             (BreakX ("t") & "tt" & Any ("acg") & "accct") or
             Cancel),
       8 => ((BreakX ("a") & "agggta" & Any ("cgt") & "a") or
             (BreakX ("t") & "t" & Any ("acg") & "taccct") or
             Cancel),
       9 => ((BreakX ("a") & "agggtaa" & Any ("cgt")) or
             (BreakX ("acg") & Any ("acg") & "ttaccct") or
             Cancel));

   type IubSub is
      record
         Element     : Pattern;
         Replacement : VString;
      end record;

   Iub : constant array (1 .. 5) of IubSub :=
     --  tHa[Nt]
     (("tHa" & Any ("Nt"), V ("<4>")),

      --  aND|caN|Ha[DS]|WaS
      ("aND" or "caN" or ("Ha" & Any ("DS")) or "WaS", V ("<3>")),

      --  a[NSt]|BY
      (("a" & Any ("NSt")) or "BY", V ("<2>")),

      --  A POSIX quantifier "*" attached to a character class means greedy
      --  matching.  In SPITBOL, a quantified character class with greed added
      --  is the realm of SPAN, and similarly of BREAK in case of negation.

      --  <[^>]*>
      ("<" & Break (">") & ">", V ("|")),

      --   \|[^|][^|]*\|
      ("|" & NotAny ("|") & Break ("|") & "|",  V ("-")));

   Seq : U.String_Access;
end DNA;
