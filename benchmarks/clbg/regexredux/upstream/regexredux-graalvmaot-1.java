/*
   The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Francois Green
*/

import java.io.*;

import java.util.*;
import java.util.Map.Entry;
import java.util.function.*;
import java.util.regex.*;

import static java.util.stream.Collectors.*;

public class regexredux {

  public static void main(String[] args) throws IOException {
    ByteArrayOutputStream baos = new ByteArrayOutputStream();
    {
        byte[] buf = new byte[65536];
        int count;
        while ((count = System.in.read(buf)) > 0) {
            baos.write(buf, 0, count);
        }
    }
    final String input = baos.toString("US-ASCII");

    final int initialLength = input.length();

    final String sequence = input.replaceAll(">.*\n|\n", "");

    final int codeLength = sequence.length();

    final List<String> variants = Arrays.asList("agggtaaa|tttaccct",
                                                "[cgt]gggtaaa|tttaccc[acg]",
                                                "a[act]ggtaaa|tttacc[agt]t",
                                                "ag[act]gtaaa|tttac[agt]ct",
                                                "agg[act]taaa|ttta[agt]cct",
                                                "aggg[acg]aaa|ttt[cgt]ccct",
                                                "agggt[cgt]aa|tt[acg]accct",
                                                "agggta[cgt]a|t[acg]taccct",
                                                "agggtaa[cgt]|[acg]ttaccct");

    BiFunction<String, String, Entry<String, Long>> counts = (v, s) -> {
       //Java 9 Matcher.results isn't off by one
      Long count = Pattern.compile(v).splitAsStream(s).count() - 1;
      return new AbstractMap.SimpleEntry<>(v, count);
    };

    final Map<String, Long> results = variants.parallelStream()
                                              .map(variant -> counts.apply(variant, sequence))
                                              .collect(toMap(Map.Entry::getKey, Map.Entry::getValue));

    variants.forEach(variant -> System.out.println(variant + " " + results.get(variant)));

    final Map<String, String> replacements = new LinkedHashMap<>();//Only works with LinkedHashMap
    {
      replacements.put("tHa[Nt]", "<4>");
      replacements.put("aND|caN|Ha[DS]|WaS", "<3>");
      replacements.put("a[NSt]|BY", "<2>");
      replacements.put("<[^>]*>", "|");
      replacements.put("\\|[^|][^|]*\\|", "-");
    }

    String buf = sequence;
    for (Map.Entry<String, String> entry : replacements.entrySet()) {
        buf = Pattern.compile(entry.getKey()).matcher(buf).replaceAll(entry.getValue());
    }

    System.out.println();
    System.out.println(initialLength);
    System.out.println(codeLength);
    System.out.println(buf.length());
  }
}
