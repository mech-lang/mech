// Runtime-regex compact form of the published D regex-redux algorithms.
import std;
void main() {
    auto content = cast(string) stdin.byChunk(1 << 20).join;
    auto initial = content.length;
    content = content.replaceAll(regex(">.*\n|\n"), "");
    auto cleaned = content.length;
    foreach (pattern; ["agggtaaa|tttaccct","[cgt]gggtaaa|tttaccc[acg]","a[act]ggtaaa|tttacc[agt]t","ag[act]gtaaa|tttac[agt]ct","agg[act]taaa|ttta[agt]cct","aggg[acg]aaa|ttt[cgt]ccct","agggt[cgt]aa|tt[acg]accct","agggta[cgt]a|t[acg]taccct","agggtaa[cgt]|[acg]ttaccct"]) writeln(pattern, " ", content.matchAll(regex(pattern)).walkLength);
    foreach (i,pattern; ["tHa[Nt]","aND|caN|Ha[DS]|WaS","a[NSt]|BY","<[^>]*>","\\|[^|][^|]*\\|"]) content = content.replaceAll(regex(pattern), ["<4>","<3>","<2>","|","-"][i]);
    writefln("\n%s\n%s\n%s", initial, cleaned, content.length); }
