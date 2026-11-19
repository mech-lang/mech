from pathlib import Path
from hashlib import sha256
import subprocess,json
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');L=Path(__file__).resolve().parent;P=W/'src/syntax/src/document/parser/canonical/base/continuation.rs';original=P.read_text();(L/'continuation-before.rs').write_text(original);s=original
s=s.replace('''    pub work: u64,
}''','''    pub work: u64,
    #[cfg(test)]
    disable_sealed_fusion: bool,
}''',1)
s=s.replace('''            work: 0,
        }
    }
    fn scanner''','''            work: 0,
            #[cfg(test)]
            disable_sealed_fusion: false,
        }
    }
    fn scanner''',1)
a=s.index('    fn call(\n');b=s.index('    fn accept_grapheme(\n',a);call=s[a:b];call=call.replace('''    ) {
        let fixed''','''    ) -> Option<Progress> {
        let fixed''',1).replace('return;','return None;')
needle='''            Spec::Classified(kind, classify) => self.push(Frame::Classified(kind, classify)),''';assert call.count(needle)==1;call=call.replace(needle,'''            Spec::Classified(kind, classify) => {
                if self.charge_sealed_followup(final_input, allowance) {
                    return self.classified(parser, kind, classify, final_input, allowance);
                }
                self.push(Frame::Classified(kind, classify));
            }''')
assert call.endswith('''        }
    }

''');call=call[:-len('''        }
    }

''')]+'''        }
        None
    }

''';s=s[:a]+call+s[b:]
a=s.index('''                Frame::Classified(kind, classify) => {''');b=s.index('''                Frame::Raw(kind) => {''',a);classified=s[a:b];a_raw=b;b_raw=s.index('''                Frame::Grapheme(kind) => {''',a_raw);raw=s[a_raw:b_raw]
# Keep the existing handlers as the only owners of classification/scanning effects.
classified_body=classified.split('=> {\n',1)[1].rsplit('                }\n',1)[0];classified_body=classified_body.replace('''                            self.push(Frame::Raw(Some(kind)));''','''                            if self.charge_sealed_followup(final_input, allowance) {
                                return self.raw(parser, Some(kind), final_input, allowance);
                            }
                            self.push(Frame::Raw(Some(kind)));''').replace('return Progress::NeedInput;','return Some(Progress::NeedInput);')
raw_body=raw.split('=> {\n',1)[1].rsplit('                }\n',1)[0].replace('return pending;','return Some(pending);')
replacement='''                Frame::Classified(kind, classify) => {
                    if let Some(progress) =
                        self.classified(parser, kind, classify, final_input, allowance)
                    {
                        return progress;
                    }
                }
                Frame::Raw(kind) => {
                    if let Some(progress) = self.raw(parser, kind, final_input, allowance) {
                        return progress;
                    }
                }
''';s=s[:a]+replacement+s[b_raw:]
needle='''                Frame::Call(rule) => self.call(parser, rule, final_input, allowance),''';assert s.count(needle)==1;s=s.replace(needle,'''                Frame::Call(rule) => {
                    if let Some(progress) = self.call(parser, rule, final_input, allowance) {
                        return progress;
                    }
                }''')
helpers='''    /// Execute the next deterministic phase without a stack round trip only
    /// when this sealed poll can pay that phase's original logical work.
    #[inline]
    fn charge_sealed_followup(&mut self, final_input: bool, allowance: &mut u64) -> bool {
        #[cfg(test)]
        if self.disable_sealed_fusion {
            return false;
        }
        if !final_input || *allowance == 0 {
            return false;
        }
        *allowance -= 1;
        self.work += 1;
        true
    }

    fn classified(
        &mut self,
        parser: &mut Parser<'_>,
        kind: SyntaxKind,
        classify: fn(char) -> bool,
        final_input: bool,
        allowance: &mut u64,
    ) -> Option<Progress> {
'''+classified_body+'''        None
    }

    fn raw(
        &mut self,
        parser: &mut Parser<'_>,
        kind: Option<SyntaxKind>,
        final_input: bool,
        allowance: &mut u64,
    ) -> Option<Progress> {
'''+raw_body+'''        None
    }

'''
needle='    fn accept_grapheme(\n';assert s.count(needle)==1;s=s.replace(needle,helpers+needle)
s+='''\n#[cfg(test)]\nmod fused_tests;\n''';P.write_text(s);subprocess.run(['rustfmt','+nightly-2026-03-03','--edition','2024','--config','skip_children=true',str(P)],check=True)
record={'source_commit':'2bc4014587fda4f31bdade02abd0cd5144d80e23','original_sha256':sha256(original.encode()).hexdigest(),'candidate_sha256':sha256(P.read_bytes()).hexdigest(),'scope':'Sealed deterministic Call/Classified/Raw chain; same handlers, per-phase charges and pending-frame boundaries. No scanner batching or Unicode assumptions.'};(L/'fusion-source.json').write_text(json.dumps(record,indent=2)+'\n');print(record)
