// D2 adaptation of bearophile's archived Benchmark Game high-level program.
import std;
size_t[string] frequencies(string sequence, size_t k) {
    size_t[string] result;
    if (sequence.length < k)
        return result;
    foreach (i; 0 .. sequence.length + 1 - k)
        result[sequence[i .. i+k]]++;
    return result;
}
void table(string sequence, size_t k) {
    auto counts = frequencies(sequence, k);
    auto keys = counts.keys;
    sort!((a,b) => counts[a] > counts[b] || (counts[a] == counts[b] && a < b))(keys);
    foreach (key; keys)
        writefln("%s %.3f", key, 100.0 * counts[key] / (sequence.length + 1 - k));
    writeln();
}
void main() {
    auto sequence = appender!string();
    bool reading;
    foreach (line; stdin.byLine()) {
        if (line.startsWith(">THREE"))
            reading = true;
        else if (reading && line.length && line[0] == '>')
            break;
        else if (reading && (!line.length || line[0] != ';'))
            sequence.put(line.toUpper);
    }
    table(sequence.data, 1);
    table(sequence.data, 2);
    foreach (query; "GGT GGTA GGTATT GGTATTTTAATT GGTATTTTAATTTATAGT".split)
        writefln("%s\t%s", frequencies(sequence.data, query.length).get(query, 0), query);
}
