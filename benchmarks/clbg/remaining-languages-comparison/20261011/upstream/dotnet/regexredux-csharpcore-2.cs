/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Michael Ganss, derived from
   Regex-Redux by Josh Goldfoot
   order variants by execution time by Anthony Lloyd
*/

using System;
using System.Threading.Tasks;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;

public static class RegexRedux
{
    public static void Main(string[] args)
    {
        var (sequences, seqLen) = Read(Console.OpenStandardInput());
        var initialLength = seqLen;

        (sequences, seqLen) = new Pcre(">.*\n|\n").Replace(sequences, seqLen, new byte[] { });

        var magicTask = Task.Run(() =>
        {
            var replacements = new (string regex, string replacement)[]
            {
                ("tHa[Nt]", "<4>"),
                ("aND|caN|Ha[DS]|WaS", "<3>"),
                ("a[NSt]|BY", "<2>"),
                ("<[^>]*>", "|"),
                ("\\|[^|][^|]*\\|", "-")
            };
            return replacements.Aggregate((seq: sequences, len: seqLen),
                (s, r) => new Pcre(r.regex).Replace(s.seq, s.len, Encoding.ASCII.GetBytes(r.replacement))).len;
        });

        var regexes = new[]
        {
            "agggtaaa|tttaccct",
            "[cgt]gggtaaa|tttaccc[acg]",
            "a[act]ggtaaa|tttacc[agt]t",
            "ag[act]gtaaa|tttac[agt]ct",
            "agg[act]taaa|ttta[agt]cct",
            "aggg[acg]aaa|ttt[cgt]ccct",
            "agggt[cgt]aa|tt[acg]accct",
            "agggta[cgt]a|t[acg]taccct",
            "agggtaa[cgt]|[acg]ttaccct"
        };

        var counts = regexes.AsParallel().AsOrdered()
            .Select(r => r + " " + new Pcre(r).Matches(sequences, seqLen).Count());

        foreach (var count in counts)
            Console.Out.WriteLine(count);

        Console.Out.WriteLine($"\n{initialLength}\n{seqLen}\n{magicTask.Result}");
    }

    static (byte[] buf, long len) Read(Stream stream)
    {
        var buf = new byte[1024 * 1024];
        var len = 0L;
        var bytesRead = 0;

        while ((bytesRead = stream.Read(buf, (int)len, buf.Length - (int)len)) > 0)
            if ((len += bytesRead) == buf.Length)
                Array.Resize(ref buf, buf.Length * 2);

        return (buf, len);
    }

    class Pcre
    {
        readonly IntPtr pcre;
        readonly IntPtr match_data;
        readonly IntPtr ovector;

        const int ErrorMsgMaxLen = 1024;
        const int PCRE2_JIT_COMPLETE = 0x00000001;
        const int PCRE2_SUBSTITUTE_GLOBAL = 0x00000100;
        const int PCRE2_NO_UTF_CHECK = 0x40000000;
        const long PCRE2_ZERO_TERMINATED = (~0L);

        string GetErrorMessage(int errorcode)
        {
            var errmsg = new StringBuilder(ErrorMsgMaxLen);
            PcreGetErrorMessage(errorcode, errmsg, ErrorMsgMaxLen);
            return errmsg.ToString();
        }

        public Pcre(string pattern)
        {
            pcre = PcreCompile(pattern, PCRE2_ZERO_TERMINATED, 0, out int errorcode, out long erroffset, IntPtr.Zero);
            if (pcre == IntPtr.Zero)
                throw new ArgumentException($@"Error compiling pattern ""{pattern}"": {GetErrorMessage(errorcode)} at offset {erroffset}");

            var ret = PcreJitCompile(pcre, PCRE2_JIT_COMPLETE);
            if (ret < 0)
                throw new ArgumentException($@"Error jit compiling pattern ""{pattern}"": {GetErrorMessage(ret)}");

            match_data = PcreMatchDataCreate(16, IntPtr.Zero);
            if (match_data == IntPtr.Zero)
                throw new ArgumentException($@"Match data could not be obtained for pattern ""{pattern}""");

            ovector = PcreGetOvectorPointer(match_data);
        }

        public IEnumerable<(long start, long end)> Matches(byte[] subject, long length)
        {
            for (var offset = 0L; Exec(subject, length, offset) >= 0; offset = Marshal.ReadInt64(ovector, 8))
                yield return (Marshal.ReadInt64(ovector), Marshal.ReadInt64(ovector, 8));
        }

        public (byte[] str, long length) Replace(byte[] subject, long length, byte[] replacement)
        {
            var outlength = length * 2;
            var output = new byte[outlength];

            unsafe
            {
                fixed (byte* s = subject, r = replacement, o = output)
                {
                    var ret = PcreSubstitute(pcre, s, length, 0L, PCRE2_SUBSTITUTE_GLOBAL | PCRE2_NO_UTF_CHECK, match_data, IntPtr.Zero,
                        r, replacement.Length, o, out outlength);
                    return (output, outlength);
                }
            }
        }

        int Exec(byte[] subject, long length, long startoffset)
        {
            unsafe
            {
                fixed (byte* b = subject)
                    return PcreJitMatch(pcre, b, length, startoffset, PCRE2_NO_UTF_CHECK, match_data, IntPtr.Zero);
            }
        }

        [DllImport("pcre2-8", EntryPoint = "pcre2_compile_8", CharSet = CharSet.Ansi)]
        extern static IntPtr PcreCompile(string pattern, long length, uint options,
            out int errorcode, out long erroroffset, IntPtr ccontext);

        [DllImport("pcre2-8", EntryPoint = "pcre2_jit_compile_8", CharSet = CharSet.Ansi)]
        extern static int PcreJitCompile(IntPtr code, uint options);

        [DllImport("pcre2-8", EntryPoint = "pcre2_jit_match_8", CharSet = CharSet.Ansi)]
        extern unsafe static int PcreJitMatch(IntPtr code, byte* subject,
            long length, long startoffset, int options, IntPtr match_data, IntPtr mcontext);

        [DllImport("pcre2-8", EntryPoint = "pcre2_match_data_create_8", CharSet = CharSet.Ansi)]
        extern unsafe static IntPtr PcreMatchDataCreate(uint ovecsize, IntPtr mcontext);

        [DllImport("pcre2-8", EntryPoint = "pcre2_get_error_message_8", CharSet = CharSet.Ansi)]
        extern unsafe static int PcreGetErrorMessage(int errorcode, StringBuilder buffer, long bufflen);

        [DllImport("pcre2-8", EntryPoint = "pcre2_get_ovector_pointer_8", CharSet = CharSet.Ansi)]
        extern unsafe static IntPtr PcreGetOvectorPointer(IntPtr match_data);

        [DllImport("pcre2-8", EntryPoint = "pcre2_substitute_8", CharSet = CharSet.Ansi)]
        extern unsafe static int PcreSubstitute(IntPtr code, byte* subject,
            long length, long startoffset, int options, IntPtr match_data, IntPtr mcontext,
            byte* replacement, long rlength, byte* outputbuffer, out long outlength);
    }
}
