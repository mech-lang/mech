/*
   The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Francois Green
*/

import java.io.*;

import java.util.concurrent.CompletableFuture;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import java.util.stream.*;

public class regexredux {

    public static void main(String[] args) throws IOException {

    var baos = new ByteArrayOutputStream();
    {
        byte[] buf = new byte[65536];
        int count;
        while ((count = System.in.read(buf)) > 0) {
        baos.write(buf, 0, count);
        }
    }
    final var input = baos.toString("US-ASCII");

    final var sequence = Pattern.compile(">.*\n|\n")
                                   .matcher(input).replaceAll("");

    final var replacements = CompletableFuture.supplyAsync(() ->
        Stream.of(
            Map.entry("tHa[Nt]", "<4>"),
            Map.entry("aND|caN|Ha[DS]|WaS", "<3>"),
            Map.entry("a[NSt]|BY", "<2>"),
            Map.entry("<[^>]*>", "|"),
            Map.entry("\\|[^|][^|]*\\|", "-")
        ).reduce(sequence,
            (buffer, e) -> Pattern.compile(e.getKey())
                                     .matcher(buffer).replaceAll(e.getValue()),
            (a, __) -> a));

    final var variants = List.of(
        "agggtaaa|tttaccct",
        "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct",
        "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct",
        "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct"
    );

    final var results = variants.parallelStream()
        .collect(Collectors.toMap(v -> v, v -> Pattern
           .compile(v).matcher(sequence).results().count()));

    variants.forEach(v -> System.out.println(v + " " + results.get(v)));

    System.out.println();
    System.out.println(input.length());
    System.out.println(sequence.length());
    System.out.println(replacements.join().length());
    }
}
