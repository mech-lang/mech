"""Use ordinary statically resolved .NET formatting for NativeAOT F# candidates."""
def adapt(text):
    substitutions={
        'printf "%.9f\\n%.9f\\n" e0 (energy system)': 'System.Console.Write("{0:F9}\\n{1:F9}\\n", e0, energy system)',
        '|> printfn "%.9f"': '|> fun value -> System.Console.WriteLine("{0:F9}", value)',
        'printfn "stretch tree of depth %d\\t check: %d" stretchDepth (bottomUpTree(stretchDepth).itemCheck())': 'System.Console.WriteLine("stretch tree of depth {0}\\t check: {1}", stretchDepth, bottomUpTree(stretchDepth).itemCheck())',
        'sprintf "%d\\t trees of depth %d\\t check: %d" iterations depth check': 'System.String.Format("{0}\\t trees of depth {1}\\t check: {2}", iterations, depth, check)',
        '(printfn "%s")': '(fun value -> System.Console.WriteLine(value:string))',
        'printfn "long lived tree of depth %d\\t check: %d" maxDepth (longLivedTree.itemCheck())': 'System.Console.WriteLine("long lived tree of depth {0}\\t check: {1}", maxDepth, longLivedTree.itemCheck())',
        'Printf.printf "%d\\nPfannkuchen(%d) = %d\\n" checksum n maxflips': 'System.Console.Write("{0}\\nPfannkuchen({1}) = {2}\\n", checksum, n, maxflips)',
        'printf "%s %i\\n" s ((regex s).Matches text).Count': 'System.Console.Write("{0} {1}\\n", s, ((regex s).Matches text).Count)',
        'printf "\\n%i\\n%i\\n%i\\n" input.Length text.Length newText.Length': 'System.Console.Write("\\n{0}\\n{1}\\n{2}\\n", input.Length, text.Length, newText.Length)',
        'printfn "%s %.3f" p.Key (100.0*float p.Value/float(dna.Length-width+1))': 'System.Console.WriteLine("{0} {1:F3}", p.Key, 100.0*float p.Value/float(dna.Length-width+1))',
        'printfn "%s %.3f" (decode p.Key width) (100.0*float p.Value/float(dna.Length-width+1))': 'System.Console.WriteLine("{0} {1:F3}", decode p.Key width, 100.0*float p.Value/float(dna.Length-width+1))',
        'printfn "%d\\t%s" total query': 'System.Console.WriteLine("{0}\\t{1}", total, query)',
        'printfn "%d\\t%s" total queries.[i]': 'System.Console.WriteLine("{0}\\t{1}", total, queries.[i])',
    }
    # Replace the longer variable name before its prefix.
    for old in sorted(substitutions,key=len,reverse=True):text=text.replace(old,substitutions[old])
    text=text.replace('|> Async.Parallel','|> fun jobs -> Async.Parallel(jobs, maxDegreeOfParallelism=8)')
    return text
