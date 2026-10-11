"""Keep upstream sources intact; make inspectable API and required-work repairs."""
import hashlib,json,re
from pathlib import Path
from fetch import metrics
D=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    manifest=json.loads((D/'manifest.json').read_text())
    rows=[]
    def add(original,source,notes):
        row=original.copy();identifier=original['id']+'-faithful'
        target=D/'sources'/(identifier+'.swift');target.write_text(source)
        row.update(id=identifier,key=row['benchmark']+':'+identifier,source=str(target),source_sha256=sha(target),
                   source_metrics=metrics(source),adaptations=original['adaptations']+notes,
                   upstream_id=original['id'],local_adaptation=True)
        rows.append(row)
    for row in manifest['programs']:
        text=Path(row['source']).read_text();identifier=row['id']
        if identifier=='knucleotide-swift-1':
            add(row,text.replace('.characters',''),['Replace obsolete String.characters views with the modern String collection API.'])
        elif identifier=='fasta-swift-1':
            text=text.replace('        let f = 1.0 / Double(IM)\n','').replace('Double(myrand) * f','Double(myrand) / Double(IM)')
            add(row,text,['Use explicit binary64 division for every fresh LCG value, retaining the original cumulative binary search.'])
        elif identifier=='fasta-swift-2':
            text=text.replace('Float','Double').replace('    let f = 1.0 / Double(IM)\n','').replace('Double(myrand) * f','Double(myrand) / Double(IM)')
            add(row,text,['Use binary64 probabilities and sample buffers, with explicit division per fresh RNG value; retain the original producer/consumer pipeline.'])
        elif identifier=='fasta-swift-3':
            text=text.replace('cprob: Int32','cprob: Double').replace('bsearch(e: Int32','bsearch(e: Double')
            text=text.replace('e: bufs[block][i]','e: Double(bufs[block][i]) / Double(IM)')
            text=text.replace('Int32(cumulative*Double(IM))','cumulative')
            add(row,text,['Replace integer-scaled cumulative cutoffs with binary64 cumulative probabilities; divide each fresh LCG value by IM before the original binary search. Keep all pipeline stages and output work.'])
        elif identifier=='fasta-swift-8':
            text=text.replace('   let len = seq.count\n   var i = 0\n   // explicit line buffer    \n   var b = ""        ',
                '   let len = seq.count\n   var cumulative = probability\n   for j in 1..<len { cumulative[j] += cumulative[j-1] }\n   var i = 0\n   // explicit line buffer    \n   var b = ""        ')
            text=text.replace('      var v = fastaRand(1.0)','      let v = fastaRand(1.0)')
            text=text.replace('         v -= probability[j]       \n         if v < 0.0 { break }', '         if v < cumulative[j] { break }')
            assert 'cumulative[j]' in text and 'v -= probability' not in text
            add(row,text,['Replace probability subtraction with a cumulative binary64 probability search. Retain every RNG update, division, sequence element, and output line.'])
        elif identifier in ('binarytrees-swift-1','binarytrees-swift-5'):
            if identifier.endswith('-1'):
                text=text.replace('let tree = createTree(maxDepth+1)','var tree = createTree(maxDepth+1)')
            else:
                text=text.replace('let tree = bottomUpTree(maxDepth+1)','var tree: Tree? = bottomUpTree(maxDepth+1)').replace('let check = itemCheck(tree)','let check = itemCheck(tree!)')
            marker='print("stretch tree of depth \\(maxDepth+1)\\t check: \\(check)")'
            assert marker in text
            text=text.replace(marker,marker+'\ntree = nil')
            add(row,text,['Release the stretch tree before creating the long-lived tree. Preserve every separately allocated node and every temporary tree/check.'])
        elif identifier in ('binarytrees-swift-2','binarytrees-swift-3','binarytrees-swift-4'):
            enum=not identifier.endswith('-4')
            check='itemCheck(tree)' if enum else 'tree!.check()'
            lifetime='itemCheck(longLivedTree)' if enum else 'longLivedTree!.check()'
            last='UINT32_MAX' if enum else 'Int.max'
            pattern=r'group.enter\(\)\nworkerQueue.async \{\n    let tree .*?\n\}\n\ngroup.enter\(\)\nworkerQueue.async \{\n    let longLivedTree .*?\n\}\n'
            replacement='do {\n    let tree = bottomUpTree(depth)\n    messages[0] = "stretch tree of depth \\(depth)\\t check: \\('+check+')"\n}\nlet longLivedTree = bottomUpTree(maxDepth)\n'
            text,count=re.subn(pattern,lambda match:replacement,text,flags=re.S)
            assert count==1,identifier
            text=text.replace('group.wait()', 'group.wait()\nmessages['+last+'] = "long lived tree of depth \\(maxDepth)\\t check: \\('+lifetime+')"')
            add(row,text,['Complete/release the stretch tree before long-lived allocation; keep the long-lived tree rooted throughout the temporary-tree work and check it afterward. Preserve each node allocation and every temporary traversal.'])
    report=dict(status='corrections-pinned',manifest_sha256=sha(D/'manifest.json'),programs=rows)
    (D/'corrections.json').write_text(json.dumps(report,indent=2)+'\n')
    for row in rows:print(row['id'],row['source_metrics']['nonblank_noncomment_lines'],'LOC')
if __name__=='__main__':main()
