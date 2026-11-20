"""Generate only the two-line ordinary-Mech search-call specialization."""
from pathlib import Path
from hashlib import sha256
import argparse
ORIGINAL_SHA256 = '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
BEFORE = '    | true => bt-advance(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, searching, 300u64, cache, pointer, previous-match)'
AFTER = '    | true => (searching?\n      | true => bt-without-prefix(bt-get-matches(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, max-length, 300u64, cache, pointer))\n      | false => bt-advance(mf, hash3, hash4, input, current-base, bits/wrapping-sub(position, current-base), max-length, false, 300u64, cache, pointer, previous-match))'
def main():
    p=argparse.ArgumentParser(__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path);a=p.parse_args();data=a.source.read_bytes();assert sha256(data).hexdigest()==ORIGINAL_SHA256;text=data.decode();assert text.count(BEFORE)==1;a.output.write_text(text.replace(BEFORE,AFTER))
if __name__=='__main__':main()
