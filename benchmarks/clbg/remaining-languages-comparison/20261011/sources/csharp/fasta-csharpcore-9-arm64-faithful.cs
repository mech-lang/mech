// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// line-by-line from Drake Diedrich's C program (#9)
// adapted to idiomatic C# by Arseniy Zlobintsev

using System.Runtime.CompilerServices;

static class Program {
    const int IM = 139968;
    const int IA = 3877;
    const int IC = 29573;
    const int SEED = 42;
    static uint seed = SEED;
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    static uint Rand() => seed = (seed * IA + IC) % IM;

    const int BUFLINES = 100;

    static ReadOnlySpan<byte> alu =>
        "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"u8 +
        "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"u8 +
        "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"u8 +
        "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"u8 +
        "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"u8 +
        "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"u8 +
        "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"u8;

    static ReadOnlySpan<byte> iub => "acgtBDHKMNRSVWY"u8;
    static ReadOnlySpan<double> iubp => [
        0.27,
        0.12,
        0.12,
        0.27,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02,
        0.02
    ];

    static ReadOnlySpan<byte> homosapiens => "acgt"u8;
    static ReadOnlySpan<double> homosapiensp => [
        0.3029549426680,
        0.1979883004921,
        0.1975473066391,
        0.3015094502008
    ];

    const int LINELEN = 60;

    static int Main(string[] args) {
        var n = args.Length > 0
            ? int.Parse(args[0]) : 1000;
        var stdout = Console.OpenStandardOutput();

        stdout.Write(">ONE Homo sapiens alu\n"u8);
        RepeatFasta(alu, n * 2);

        stdout.Write(">TWO IUB ambiguity codes\n"u8);
        RandomFasta(iub, iubp, n * 3);

        stdout.Write(">THREE Homo sapiens frequency\n"u8);
        RandomFasta(homosapiens, homosapiensp, n * 5);
        return 0;
    }

    static void RepeatFasta(ReadOnlySpan<byte> seq, int n) {
        var len = seq.Length;
        var buflen1 = len + LINELEN;
        var buffer1 = new byte[buflen1].AsSpan();
        int i;
        if (LINELEN < len) {
            seq[..len].CopyTo(buffer1);
            seq[..LINELEN].CopyTo(buffer1[len..]);
        }
        else {
            for (i = 0; i < LINELEN / len; i++) {
                seq[..len].CopyTo(buffer1[(i * len)..]);
            }
            seq[..(n - i * n)].CopyTo(buffer1[(i * len)..]);
        }

        var buflen2 = (LINELEN + 1) * len;
        var buffer2 = new byte[buflen2].AsSpan();
        for (i = 0; i < len; i++) {
            buffer1
                .Slice(i * LINELEN % len, LINELEN)
                .CopyTo(buffer2[(i * (LINELEN + 1))..]);
            buffer2[(i + 1) * (LINELEN + 1) - 1] = (byte)'\n';
        }

        var stdout = Console.OpenStandardOutput();
        var wholeBuffers = n / (len * LINELEN);
        for (i = 0; i < wholeBuffers; i++) {
            stdout.Write(buffer2);
        }

        var dataRemaining = n - wholeBuffers * len * LINELEN;
        var embeddedNewlines = dataRemaining / LINELEN;
        stdout.Write(buffer2[..(dataRemaining + embeddedNewlines)]);

        if (n % LINELEN != 0) stdout.Write("\n"u8);
    }

    static double[] BuildCumulative(ReadOnlySpan<double> probability) {
        var cumulative = probability.ToArray();
        for (int i = 1; i < cumulative.Length; i++)
            cumulative[i] += cumulative[i - 1];
        return cumulative;
    }
    static byte Lookup(ReadOnlySpan<byte> symbols, double[] cumulative, uint random) {
        double r = (double)random / IM;
        int i = 0;
        while (r >= cumulative[i] && i + 1 < cumulative.Length) i++;
        return symbols[i];
    }

    static byte[] BufferWithLinebreaks(int lines) {
        var buffer = new byte[(LINELEN + 1) * lines];
        for (var i = 0; i < lines; i++) {
            buffer[i * (LINELEN + 1) + LINELEN] = (byte)'\n';
        }
        return buffer;
    }

    static void RandomFasta(
        ReadOnlySpan<byte> symb,
        ReadOnlySpan<double> probability,
        int n
    ) {
        int i, j, k;
        var cumulative = BuildCumulative(probability);
        var buffer = BufferWithLinebreaks(BUFLINES);
        var buffers = n / LINELEN / BUFLINES;
        var stdout = Console.OpenStandardOutput();
        for (i = 0; i < buffers; i++) {
            for (j = 0; j < BUFLINES; j++) {
                for (k = 0; k < LINELEN; k++) {
                    var v = Rand();
                    buffer[j * (LINELEN + 1) + k] = Lookup(symb, cumulative, v);
                }
            }
            stdout.Write(buffer);
        }

        var lines = n / LINELEN - buffers * BUFLINES;
        for (j = 0; j < lines; j++) {
            for (k = 0; k < LINELEN; k++) {
                var v = Rand();
                buffer[j * (LINELEN + 1) + k] = Lookup(symb, cumulative, v);
            }
        }
        var partials = n - LINELEN * lines - buffers * BUFLINES * LINELEN;
        for (k = 0; k < partials; k++) {
            var v = Rand();
            buffer[lines * (LINELEN + 1) + k] = Lookup(symb, cumulative, v);
        }
        stdout.Write(buffer.AsSpan(..(lines * (LINELEN + 1) + partials)));

        if (n % LINELEN != 0) stdout.Write("\n"u8);
    }
}
