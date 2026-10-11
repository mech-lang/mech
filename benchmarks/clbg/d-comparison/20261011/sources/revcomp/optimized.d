// Buffered lookup-table adaptation of archived D reverse-complement sources.
import std;
void main() {
    char[256] complement;
    foreach (i,c; "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy")
        complement[cast(ubyte)c] = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"[i];
    auto input = cast(char[])stdin.byChunk(1 << 20).join;
    auto output = appender!(char[])();
    output.reserve(input.length);
    size_t start;
    while (start < input.length) {
        auto headerEnd = start;
        while (headerEnd < input.length && input[headerEnd] != '\n')
            headerEnd++;
        output.put(input[start .. min(headerEnd+1,input.length)]);
        auto sequenceStart = min(headerEnd+1,input.length);
        auto sequenceEnd = sequenceStart;
        while (sequenceEnd < input.length && input[sequenceEnd] != '>')
            sequenceEnd++;
        size_t column;
        for (auto i=sequenceEnd; i > sequenceStart;) {
            auto c = input[--i];
            if (c == '\n' || c == '\r')
                continue;
            output.put(complement[cast(ubyte)c]);
            if (++column == 60) {
                output.put('\n');
                column = 0;
            }
        }
        if (column)
            output.put('\n');
        start = sequenceEnd;
    }
    stdout.rawWrite(output.data);
}
