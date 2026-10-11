"""Compile separately labelled native-ARM .NET portability/work corrections."""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import re
import subprocess
from dotnet_fetch import D, C, sha
from metrics import normalize, fixtures

SDK=json.loads((D/'support/dotnet-sdk.json').read_text())
DOTNET=SDK['path']
ENV=os.environ|{'DOTNET_ROOT':str(Path(DOTNET).parent),'DOTNET_CLI_HOME':str(D/'support/dotnet-home'),
    'DOTNET_CLI_TELEMETRY_OPTOUT':'1','DOTNET_NOLOGO':'1','DOTNET_SKIP_FIRST_TIME_EXPERIENCE':'1',
    'DOTNET_PROCESSOR_COUNT':'8','DOTNET_GCHeapCount':'8','DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE':'true',
    'NUGET_PACKAGES':str(D/'support/nuget')}

def portable(row):
    src=Path(row['original_source']).read_text().lstrip('\ufeff')
    text=src
    notes=[]
    name=row['id']
    if re.search(r'(?m)^not accepted',text):
        raise ValueError('Official source page identifies this program as not accepted.')
    if row['benchmark']=='pidigits':
        text=re.sub(r'"(?:lib)?gmp(?:\.so(?:\.\d+)*)?"','"/opt/homebrew/opt/gmp/lib/libgmp.10.dylib"',text)
        if text!=src:notes.append('Resolve the same general GMP API using the native macOS library; application interop declarations count toward LOC.')
        if name=='pidigits-csharpcore-6':
            text=text.replace('stackalloc char[(10 + 1 + 1 + args[0].Length) * n]', 'stackalloc char[((n + 9) / 10) * (14 + args[0].Length)]')
            start=text.index('        // we have to use Console.Out')
            text=text[:start]+'''        if (n % 10 != 0) {
            for (int pad = n % 10; pad < 10; pad++) *builder++ = ' ';
            *builder++ = '\\t';
            *builder++ = ':';
            n.TryFormat(new Span<char>(builder, args[0].Length), out var written);
            builder += written;
            *builder++ = '\\n';
        }

'''+text[start:]
            text=text.replace('return GmpBindings.mpz_get_ui(temp1);','return (int)GmpBindings.mpz_get_ui(temp1);')
            text=text.replace('extern static int mpz_get_ui','extern static ulong mpz_get_ui')
            for arg in ['uint multiplicand','int newValue','int multiplicand','int subtrahend']:
                text=text.replace(arg,'ulong '+arg.split()[1])
            text=text.replace('&accumulator, k2);','&accumulator, (ulong)k2);').replace('&denominator, k2);','&denominator, (ulong)k2);')
            text=text.replace('&numerator, k);','&numerator, (ulong)k);').replace('&denominator, digit);','&denominator, (ulong)digit);').replace('numerator, nth);','numerator, (ulong)nth);')
            notes.append('Keep both pi quotients for every tested candidate and the same generic impossible-candidate guard used by Mech/Rust; pad incomplete rows and use the macOS unsigned-long GMP ABI.')
        if name=='pidigits-fsharpcore-6':
            text=text.replace('int _value)','uint64 _value)').replace('extern int private mpzGetUi','extern uint64 private mpzGetUi')
            text=re.sub(r'(mpz(?:InitSetUi|MulUi|SubmulUi|AddmulUi)\([^()\n]+,\s*)(\d+)(\))',r'\g<1>\g<2>UL\3',text)
            text=text.replace('let k2 = k * 2 + 1','let k2 = k * 2UL + 1UL')
            text=text.replace('extractDigit 3','extractDigit 3UL').replace('extractDigit 4','extractDigit 4UL')
            text=text.replace('nextTerm k\n','nextTerm (uint64 k)\n')
            end=text.index('\n    0',text.index('    while n > 0 do'))
            text=text[:end]+'''\n    if i > 0 then
        out.Write(bytes, 0, i)
        let suffix = System.Text.Encoding.ASCII.GetBytes(System.String(' ',10-i)+"\\t:"+args.[0]+"\\n")
        out.Write(suffix, 0, suffix.Length)
'''+text[end:]
            notes.append('Retain both pi quotients for every tested candidate, preserve the same generic impossible-candidate guard used by Mech/Rust, use the full native GMP unsigned-long ABI, and pad partial output rows.')
    if row['benchmark']=='fasta':
        if row['language']=='csharp':
            text=re.sub(r'\bfloat\b','double',text)
            text=re.sub(r'(?<=\d)[fF]\b','',text)
            if name.endswith('-1'):
                text=text.replace('this.p = (p * IM);','this.p = p;').replace('SelectRandom(Frequency[] a, int r)','SelectRandom(Frequency[] a, double r)')
                text=text.replace('SelectRandom(a, (int)rnd[i + j])','SelectRandom(a, (double)rnd[i + j] / IM)')
            if name.endswith('-5'):
                text=text.replace('var p = rnds[i] * FIM;','var p = (double)rnds[i] / IM;')
            if name.endswith('-9'):
                start=text.index('    static byte[] BuildHash(')
                end=text.index('    static byte[] BufferWithLinebreaks',start)
                text=text[:start]+'''    static double[] BuildCumulative(ReadOnlySpan<double> probability) {
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

'''+text[end:]
                text=text.replace('var hash = BuildHash(symb, probability);','var cumulative = BuildCumulative(probability);').replace('hash[v]','Lookup(symb, cumulative, v)')
                notes.append('Replace the RNG-state output lookup table with a cumulative binary64 search and a fresh binary64 division for every generated symbol; keep all LCG steps and buffered output.')
        else:
            text=text.replace('1.0/139968.0 * float rnds.[i]','float rnds.[i] / 139968.0')
        if text!=src:notes.append('Use binary64 probabilities and explicit division per generated random symbol, matching the full cohort.')
    if row['benchmark']=='nbody':
        if name in ('nbody-csharpcore-6','nbody-csharpcore-7'):
            for method in ('Add','Subtract','Multiply','Divide'):
                text=text.replace('Avx.'+method,'Vector256.'+method)
            text=text.replace('Sse2.Add','Vector128.Add')
            notes.append('Use portable .NET binary64 SIMD operations, lowered to native ARM vectors, in place of x86 AVX/SSE arithmetic; preserve the original algorithm and state.')
        if name in ('nbody-fsharpcore-5','nbody-fsharpcore-6'):
            for method in ('Add','Subtract','Multiply','Divide'):
                text=text.replace('Avx.'+method,'Vector256.'+method)
            notes.append('Use portable .NET Vector256 binary64 arithmetic, which splits into native ARM vectors, in place of AVX arithmetic.')
        if name in ('nbody-csharpcore-4','nbody-csharpcore-9'):
            raise ValueError('This original requires x86-specific packing and a float32 reciprocal-square-root seed; retained upstream. Native binary64 SIMD candidates are measured separately.')
    if name=='spectralnorm-csharpcore-5':
        text=text.replace('Sse2.LoadVector128','Vector128.Load').replace('Sse2.Add','Vector128.Add').replace('Sse2.Divide','Vector128.Divide')
        text=text.replace('Sse3.HorizontalAdd(sum, sum)','Vector128.Create(sum.GetElement(0) + sum.GetElement(1))')
        text=text.replace('j < n; j += 2','j + 1 < n; j += 2')
        text=text.replace('Unsafe.WriteUnaligned(outv + i, value);',
            'if ((n & 1) != 0) value += v[n - 1] / A(i, n - 1);\n                Unsafe.WriteUnaligned(outv + i, value);',1)
        position=text.index('private static void mult_Atv')
        text=text[:position]+text[position:].replace('Unsafe.WriteUnaligned(outv + i, value);',
            'if ((n & 1) != 0) value += v[n - 1] / A(n - 1, i);\n                Unsafe.WriteUnaligned(outv + i, value);',1)
        notes.append('Replace x86 load/arithmetic/horizontal-add intrinsics with the equivalent portable binary64 Vector128 operations for ARM64.')
        notes.append('Handle the final scalar column at odd sizes without an out-of-bounds vector load.')
    if row['benchmark']=='knucleotide':
        if row['language']=='fsharp':
            text=re.sub(r'(?m)^(\s*)\|> Seq.skip 1\s*$',lambda m:m[0]+'\n'+m[1]+'|> Seq.takeWhile (fun x -> not (x.StartsWith(">")))',text)
            if name.endswith('-1'):
                text=text.replace('|> List.sortBy (snd >> (~-))','|> List.sortBy (fun (s,c) -> (-c,s))')
                text=text.replace('|> String.concat ""','|> String.concat ""\n        |> fun s -> s.ToLowerInvariant()')
            if name.endswith(('-3','-4')):
                text=text.replace('let mutable h = 0','let mutable h = 0UL').replace('let mutable hash = 0','let mutable hash = 0UL')
                text=text.replace('h <- h * 31 + int dna.[k + i]','h <- (h <<< 2) ||| uint64 ((int dna.[k + i] &&& 6) >>> 1)')
                text=text.replace('hash <- hash * 31 + int (readDNA (k + i))','hash <- (hash <<< 2) ||| uint64 ((int (readDNA (k + i)) &&& 6) >>> 1)')
                text=text.replace('Dictionary<int,','Dictionary<uint64,').replace('KeyValuePair<int,','KeyValuePair<uint64,')
                notes.append('Use complete collision-free two-bit keys through all 36 bits of the eighteen-base windows in the same growable dictionaries; every overlapping window still updates a map.')
            if name.endswith(('-2','-5','-6')):
                raise ValueError('Original selectively updates longer-width maps only when the final nucleotide matches a requested query. Full-map locally authored variants replace this reduced work.')
        else:
            text=text.replace('counts.Sort((a, b) => b.Value.CompareTo(a.Value));',
                'counts.Sort((a, b) => b.Value != a.Value ? b.Value.CompareTo(a.Value) : StringComparer.Ordinal.Compare(a.Key, b.Key));')
            if name.endswith('-1'):
                text=text.replace('using Microsoft.Collections.Extensions;','using System.Runtime.InteropServices;')
                text=text.replace('DictionarySlim<','Dictionary<')
                text=re.sub(r'(\w+)\.GetOrAddValueRef\(([^()]*)\)',r'CollectionsMarshal.GetValueRefOrAddDefault(\1, \2, out _)',text)
                notes.append('Use the ordinary .NET Dictionary and CollectionsMarshal by-reference insertion API in place of the retired experimental DictionarySlim package.')
                text=text.replace('Count(18, 34359738367,','Count(18, 17179869183,').replace('Count(12, 8388607,','Count(12, 4194303,')
                text=text.replace("tonum['\\n'] = 255;", "tonum['\\r'] = 255; tonum['\\n'] = 255;")
                notes.append('Keep exactly the preceding 2*(width-1) rolling bits before adding a base, and ignore CR as well as LF in FASTA data.')
                start=text.index('    public static void LoadThreeData()')
                end=text.index('    [MethodImpl(',start)
                text=text[:start]+'''    public static void LoadThreeData() {
        using var input = new StreamReader(Console.OpenStandardInput(), Encoding.ASCII);
        bool selected = false;
        var buffer = new byte[BLOCK_SIZE];
        int offset = 0;
        string line;
        while ((line = input.ReadLine()) != null) {
            if (line.StartsWith(">")) {
                if (selected) break;
                selected = line.StartsWith(">THREE");
            } else if (selected) {
                foreach (byte symbol in Encoding.ASCII.GetBytes(line.Trim())) {
                    buffer[offset++] = symbol;
                    if (offset == BLOCK_SIZE) {
                        threeBlocks.Add(buffer);
                        buffer = new byte[BLOCK_SIZE];
                        offset = 0;
                    }
                }
            }
        }
        threeStart = 0;
        threeEnd = offset;
        threeBlocks.Add(buffer);
        if (threeBlocks.Count == 1) {
            for (int i = offset; i < buffer.Length; i++) buffer[i] = 255;
            threeBlocks.Add(Array.Empty<byte>());
            threeEnd = 0;
        }
    }

'''+text[end:]
                text=text.replace('freq.OrderByDescending(i => i.Value)','freq.OrderByDescending(i => i.Value).ThenBy(i => i.Key)')
                notes.append('Use a standard line reader to stop at the next record even in a short input buffer; retain buffered rolling maps and canonical lexical ties.')
        if text!=src:notes.append('Retain exactly the selected FASTA record, normalize nucleotide case, include all overlapping windows, and use lexical frequency ties where applicable.')
    if row['benchmark']=='binarytrees':
        if 'struct TreeNode' in text:
            raise ValueError('Struct/optional-next source does not allocate distinct equal-size leaves and interior nodes.')
        notes.append('Source review: each leaf/interior node is a distinct heap object with the same two child fields. The long-lived root is constructed before temporary loops and traversed afterward.')
    if name=='mandelbrot-csharpcore-2':
        text=text.replace('((bits << (8 - xcounter)) ^ 0xff)', '(((bits << (8 - xcounter)) ^ 0xff) & (0xff << (8 - xcounter)))')
        notes.append('Zero unused low padding bits in a partial PBM byte; preserve all actual pixel iterations.')
    if name in ('mandelbrot-fsharpcore-1','mandelbrot-fsharpcore-6'):
        text=text.replace('let lineLength = size >>> 3','let lineLength = (size + 7) >>> 3')
        text=text.replace('Array.zeroCreate (size+2)','Array.zeroCreate (lineLength*8+2)')
        text=text.replace('if i<size then','if i<lineLength*8 then')
        text=text.replace('            |> NativePtr.set pdata (y*lineLength+x)',
            '            |> fun value -> if x=lineLength-1 && size%8<>0 then value &&& byte(255 <<< (8-size%8)) else value\n            |> NativePtr.set pdata (y*lineLength+x)')
        notes.append('Allocate complete padded rows, initialize every loaded SIMD lane, and zero unused final bits at arbitrary widths.')
    if name=='spectralnorm-fsharpcore-6':
        text=text.replace('let inline mult (v:', 'let inline mult length (v:')
        text=text.replace('out[i] <- hadd sum0 sum1','let row = hadd sum0 sum1\n        out[i] <- if 2*i+1 < length then row else Vector128.Create(row.GetElement(0),0.0)')
        text=text.replace('let multAtAv v out tmp =','let multAtAv length v out tmp =')
        text=text.replace('mult v tmp a','mult length v tmp a').replace('mult tmp out (fun','mult length tmp out (fun')
        text=text.replace('let n = (try int argv[0] with _ -> 500) / 2', 'let length = try int argv[0] with _ -> 500\n    let n = (length+1) / 2')
        text=text.replace('let v = Array.zeroCreate n','if length%2 <> 0 then u[n-1] <- Vector128.Create(1.0,0.0)\n    let v = Array.zeroCreate n')
        text=text.replace('multAtAv u v tmp','multAtAv length u v tmp').replace('multAtAv v u tmp','multAtAv length v u tmp')
        notes.append('Retain the original matrix dimension with a masked final vector lane at odd sizes instead of truncating it to an even value.')
    if name=='regexredux-csharpcore-2':
        text=text.replace('"pcre2-8"','"/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib"')
        text=text.replace('o, out outlength','o, ref outlength').replace('out long outlength);','ref long outlength);')
        notes.append('Pin the native PCRE2 library and use an in/out output-size pointer in the ordinary replacement binding.')
    if name=='regexredux-csharpcore-4':
        text=text.replace('int[][] sums = Chunks(sequence).AsParallel().Select(CountRegexes).ToArray();',
            'int[] sums = Variants.variantsCopy().AsParallel().AsOrdered().Select(pattern => Regex.Matches(sequence, pattern).Count).ToArray();')
        text=text.replace('sums.Sum(a => a[i])','sums[i]')
        notes.append('Count each complete sequence in parallel by pattern; the original independent sequence chunks lost cross-boundary matches.')
    if name=='regexredux-fsharpcore-4':
        start=text.index('text.ToCharArray()')
        text=text[:start]+'''text
|> applyPatterns
|> String.length
|> printfn "\\n%i\\n%i\\n%i" input.Length text.Length
'''
        notes.append('Run the ordered substitution chain over the entire sequence; chunking substitutions changes cross-boundary matches.')
    if name=='regexredux-fsharpcore-7':
        text=text.replace('open PCRE','open System.Text.RegularExpressions')
        text=text.replace('PcreRegex(s,PcreOptions.Compiled|||PcreOptions.NoUtfCheck)','Regex(s,RegexOptions.Compiled)')
        text=re.sub(r'^let settings = .*\n','',text,flags=re.M)
        text=text.replace('.Matches(text,settings)','.Matches(text)')
        text=text.replace('(fun s (pattern, replace) ->','(fun (s:string) (pattern:string, replace:string) ->')
        notes.append('Use the standard .NET regex API for the identical patterns and ordered substitutions instead of the unavailable third-party PCRE.NET package.')
    if name in ('fannkuchredux-csharpcore-5','revcomp-csharpcore-9'):
        text=text.replace('using System.Runtime.Intrinsics.X86;','using System.Runtime.Intrinsics.Arm;')
        for method in ('Add','Subtract'):
            text=text.replace('Sse2.'+method,'Vector128.'+method)
        text=text.replace('Sse2.CompareEqual','Vector128.Equals').replace('Sse2.CompareGreaterThan','Vector128.GreaterThan')
        text=text.replace('Sse2.AndNot','ArmCompat.AndNot').replace('Sse2.And','Vector128.BitwiseAnd')
        text=text.replace('Sse2.MoveMask','ArmCompat.MoveMask').replace('Sse2.ConvertToInt32','ArmCompat.Int32')
        text=text.replace('Sse2.ShiftRightLogical128BitLane(ramp, 1)','AdvSimd.ExtractVector128(ramp, Vector128<byte>.Zero, 1)')
        text=text.replace('Ssse3.AlignRight(countVector, v2, 15)','AdvSimd.ExtractVector128(v2, countVector, 15)')
        text=text.replace('Ssse3.Shuffle','ArmCompat.Shuffle').replace('Sse41.BlendVariable','ArmCompat.Blend')
        text=text.replace('Sse2.LoadVector128','Vector128.Load').replace('Avx.LoadDquVector256','Vector256.Load')
        text=re.sub(r'Sse2.Store\(([^,]+),\s*([^;]+)\);',r'Vector128.Store(\2, \1);',text)
        text=re.sub(r'Avx.Store\(([^,]+),\s*(Vector256.Load\([^;]+)\);',r'Vector256.Store(\2, \1);',text)
        text+='''
static class ArmCompat {
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)]
    internal static Vector128<byte> Shuffle(Vector128<byte> a, Vector128<byte> index)
        => Vector128.Shuffle(a, index & Vector128.Create((byte)0x8f));
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)]
    internal static Vector128<byte> Blend(Vector128<byte> a, Vector128<byte> b, Vector128<byte> mask)
        => Vector128.ConditionalSelect(Vector128.LessThan(mask.AsSByte(), Vector128<sbyte>.Zero).AsByte(), b, a);
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)]
    internal static Vector128<byte> AndNot(Vector128<byte> a, Vector128<byte> b) => ~a & b;
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)]
    internal static int MoveMask(Vector128<byte> a) => (int)Vector128.ExtractMostSignificantBits(a);
    [System.Runtime.CompilerServices.MethodImpl(System.Runtime.CompilerServices.MethodImplOptions.AggressiveInlining)]
    internal static int Int32(Vector128<int> a) => a.GetElement(0);
}
'''
        if name=='fannkuchredux-csharpcore-5':
            text=text.replace('_blockCount = maxBlocks * nThreads;', '_blockCount = Math.Min(maxBlocks * nThreads, factorials[_n]);\n        while (_blockCount > 1 && (factorials[_n] % _blockCount != 0 || (factorials[_n] / _blockCount & 1) != 0)) _blockCount--;')
            text=text.replace('if (maxFlips > _maxFlips) _maxFlips = (byte)maxFlips;\n        if (maxFlips > _maxFlips) _maxFlips = (byte)maxFlips;', 'lock (_factorials) {\n            if (maxFlips > _maxFlips) _maxFlips = (byte)maxFlips;\n        }')
            notes.append('Choose complete even-length factorial blocks for all sizes and preserve the maximum with a final standard lock. Every permutation, checksum term and maximum remains computed.')
        notes.append('Port byte shuffles, masks and loads/stores to portable .NET vectors and ARM extraction. All compatibility helpers are C# application source and count toward LOC.')
    if row['language']=='fsharp':
        # Modern F# reserves additional words, but the exact historical APIs remain available.
        text=text.replace('\r\n','\n')
    out=dict(row)
    ident=name+'-arm64-faithful' if text!=src else name
    path=D/'sources'/row['language']/(ident+Path(row['source']).suffix)
    if text!=src:path.write_text(text)
    out.update(id=ident,key=row['benchmark']+':'+ident,source=str(path),source_sha256=sha(path),
        adaptations=notes,local_adaptation=text!=src,**normalize(text,row['language']))
    return out

def build(row,aot=False):
    row=dict(row)
    if aot and row['language']=='fsharp':
        from dotnet_static_output import adapt
        original=Path(row['source'])
        text=adapt(original.read_text())
        if text!=original.read_text():
            row['id']+='-static-output'
            path=original.with_name(row['id']+'.fs');path.write_text(text)
            row.update(source=str(path),source_sha256=sha(path),**normalize(text,'fsharp'))
            row['adaptations']=row.get('adaptations',[])+['Use ordinary statically resolved .NET number/text formatting, because this FSharp.Core printf path invokes generic methods through reflection that NativeAOT cannot create. All changed application output expressions count. Limit async scheduling to eight workers.']
    ident=row['id']+('-aot' if aot else '-jit')
    project=D/'support/dotnet-builds'/ident
    project.mkdir(parents=True,exist_ok=True)
    fs=row['language']=='fsharp'
    ext='fsproj' if fs else 'csproj'
    manifest=project/('app.'+ext)
    properties='''<OutputType>Exe</OutputType><TargetFramework>net10.0</TargetFramework>
<ImplicitUsings>enable</ImplicitUsings><Nullable>disable</Nullable><AllowUnsafeBlocks>true</AllowUnsafeBlocks>
<Optimize>true</Optimize><ServerGarbageCollection>true</ServerGarbageCollection><ConcurrentGarbageCollection>true</ConcurrentGarbageCollection>
<InvariantGlobalization>true</InvariantGlobalization><EnableDefaultCompileItems>false</EnableDefaultCompileItems>
<UseSharedCompilation>false</UseSharedCompilation><NuGetAudit>false</NuGetAudit><CheckForOverflowUnderflow>false</CheckForOverflowUnderflow>
<NoWarn>FS3370;FS3511;CS8981;CS8600</NoWarn>'''
    if aot:properties+='<PublishAot>true</PublishAot><OptimizationPreference>Speed</OptimizationPreference><IlcOptimizationPreference>Speed</IlcOptimizationPreference>'
    manifest.write_text('<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup>'+properties+'</PropertyGroup><ItemGroup><Compile Include="'+row['source']+'" /></ItemGroup></Project>\n')
    output=D/'binaries'/ident
    output.mkdir(exist_ok=True)
    cmd=[DOTNET,'publish' if aot else 'build',str(manifest),'-c','Release','-o',str(output),'--disable-build-servers','-v','quiet']
    if aot:cmd+=['-r','osx-arm64','--self-contained','true']
    log=D/'results'/(ident+'.build.log')
    record=project/'build-record.json'
    previous=json.loads(record.read_text()) if record.exists() else None
    if previous and previous['source_sha256']==row['source_sha256'] and previous['build_command']==cmd and previous['build_status']=='passed':
        if all(Path(p).exists() and sha(p)==h for p,h in previous['artifact_pins'].items()):
            return dict(previous,**normalize(Path(row['source']).read_text(),row['language']))
    result=subprocess.run(cmd,cwd=project,env=ENV,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=900)
    log.write_text(result.stdout)
    row.update(id=ident,key=row['benchmark']+':'+ident,build_command=cmd,build_status='passed' if result.returncode==0 else 'failed',
        build_log=str(log),language=row['language']+('-aot' if aot else ''),execution_mode='aot' if aot else 'jit')
    if result.returncode==0:
        binary=output/('app' if aot else 'app.dll')
        row.update(binary=str(binary),binary_sha256=sha(binary),
            command=([str(binary)] if aot else [DOTNET,str(binary)])+([] if row['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']),
            runtime_environment={'DOTNET_PROCESSOR_COUNT':'8','DOTNET_GCHeapCount':'8'},
            artifact_pins={str(p):sha(p) for p in output.iterdir() if p.is_file()})
        row['algorithm']=dict(strict=True,native_benchmark_kernel=False,
            arithmetic='binary64',work='Unchanged full benchmark workload; application code and all application FFI/helpers count toward LOC.')
    print('BUILD',ident,row['build_status'],flush=True)
    if result.returncode:print(result.stdout[-900:],flush=True)
    record.write_text(json.dumps(row,indent=2)+'\n')
    return row

def main():
    fixtures()
    ap=argparse.ArgumentParser()
    ap.add_argument('--aot',action='store_true')
    ap.add_argument('--only')
    args=ap.parse_args()
    report=json.loads((D/'dotnet_sources.json').read_text())
    rows,excluded=[],[]
    for row in report['programs']:
        if args.only and row['id'] not in args.only.split(','):continue
        try:rows.append(portable(row))
        except Exception as e:excluded.append(dict(id=row['id'],benchmark=row['benchmark'],language=row['language'],reason=str(e),source=row['original_source'],sha256=row['original_source_sha256']))
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        built=list(pool.map(lambda r:build(r,args.aot),rows))
    result=dict(status='built',sdk=SDK,programs=built,excluded=excluded)
    path=D/('dotnet_aot_builds.json' if args.aot else 'dotnet_jit_builds.json')
    if args.only and path.exists():
        old=json.loads(path.read_text())
        affected={r['original_source'] for r in built if r.get('original_source')}
        by_key={r['key']:r for r in old['programs'] if r.get('original_source') not in affected}
        by_key.update({r['key']:r for r in built})
        result['programs']=list(by_key.values())
        result['excluded']=old.get('excluded',[])+excluded
    path.write_text(json.dumps(result,indent=2)+'\n')
    print('BUILT',sum(x['build_status']=='passed' for x in built),'of',len(built),flush=True)

if __name__=='__main__':main()
