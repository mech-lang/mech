// D2 adaptation of bearophile's high-level archived Benchmark Game source.
import std;
immutable letters = "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy";
immutable complements = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR";
void show(char[] sequence) {
    auto result = sequence.map!(c => cast(char)complements[letters.countUntil(c)]).array.reverse;
    foreach (i; iota(0, result.length, 60))
        writeln(result[i .. min(i+60,result.length)]);
}
void main() {
    auto sequence = appender!(char[])();
    foreach (line; stdin.byLine()) {
        if (line.length && line[0] == '>') {
            show(sequence.data);
            writeln(line);
            sequence.clear;
        } else if (!line.length || line[0] != ';')
            sequence.put(line);
    }
    show(sequence.data);
}
