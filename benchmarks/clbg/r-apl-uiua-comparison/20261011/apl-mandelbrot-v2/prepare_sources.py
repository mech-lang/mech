"""Write additive APL active-pixel candidates; never touch frozen originals."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
G = HERE.parent

COMPACT = '''⍝ Counted APL: compact row with stable compression of live pixel indices.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;Y;CR;CI;ZR;ZI;R2;I2;Alive;Index;Keep;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 CR←¯1.5+2.0×(⍳N)÷N
 Pad←8×⌈N÷8
 Y←0
Row:→(Y≥N)/0
 CI←¯1+2.0×Y÷N
 ZR←N⍴0.0
 ZI←N⍴0.0
 Alive←N⍴1
 Index←⍳N
 Iter←0
Loop:→(Iter≥50)/Emit
 R2←ZR×ZR
 I2←ZI×ZI
 ZI←CI+2×ZR×ZI
 ZR←CR[Index]+R2-I2
 Keep←4≥(ZR×ZR)+ZI×ZI
 Alive[Index]←Keep
 Index←Keep/Index
 ZR←Keep/ZR
 ZI←Keep/ZI
 Iter←Iter+1
 →(0<⍴Index)/Loop
Emit:Bits←Pad↑Alive
 Bytes←2⊥⍉((Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Y←Y+1
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
'''

OPTIMIZED = '''⍝ Counted APL: sixteen rows with stable compression of live pixel indices.
⎕IO←0
⎕CT←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
∇ Run N;Y;Offset;Rows;CR;CI;ZR;ZI;R2;I2;Alive;Index;Keep;Iter;Bits;Bytes;Pad
 ⊣(('P4',(⎕UCS 10),'%d %d',⎕UCS 10) N N)⎕FIO[22]OUT
 Pad←8×⌈N÷8
 Offset←0
Row:→(Offset≥N)/0
 Rows←16⌊N-Offset
 Y←Offset+⍳Rows
 CR←(Rows×N)⍴¯1.5+2.0×(⍳N)÷N
 CI←,(¯1+2.0×Y÷N)∘.+N⍴0.0
 ZR←(Rows×N)⍴0.0
 ZI←(Rows×N)⍴0.0
 Alive←(Rows×N)⍴1
 Index←⍳Rows×N
 Iter←0
Loop:→(Iter≥50)/Emit
 R2←ZR×ZR
 I2←ZI×ZI
 ZI←CI[Index]+2×ZR×ZI
 ZR←CR[Index]+R2-I2
 Keep←4≥(ZR×ZR)+ZI×ZI
 Alive[Index]←Keep
 Index←Keep/Index
 ZR←Keep/ZR
 ZI←Keep/ZI
 Iter←Iter+1
 →(0<⍴Index)/Loop
Emit:Bits←(Rows,Pad)↑(Rows,N)⍴Alive
 Bytes←,⍉2⊥⍉(Rows,(Pad÷8),8)⍴Bits
 ⊣Bytes⎕FIO[7]OUT
 Offset←Offset+Rows
 →Row
∇
Run ⍎,⊃¯1↑⎕ARG
⊣⎕FIO[4]OUT
)OFF
'''


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def loc(text):
    assert all('⋄' not in line and '◊' not in line for line in text.splitlines())
    assert all(line.split('⍝', 1)[0].count('←') <= 1 for line in text.splitlines())
    return sum(bool(line.strip()) and not line.lstrip().startswith('⍝')
               and line.strip() != '∇' for line in text.splitlines())


def main():
    sources = HERE / 'sources'
    sources.mkdir(parents=True, exist_ok=True)
    old = json.loads((G / 'apl-manifest.json').read_text())
    preserved = {str(G / 'apl-manifest.json'): sha(G / 'apl-manifest.json')}
    for row in old['programs']:
        if row['benchmark'] == 'mandelbrot':
            preserved[row['source']] = row['source_sha256']
    rows = []
    for variant, text in [('compact', COMPACT), ('optimized', OPTIMIZED)]:
        path = sources / ('mandelbrot.' + variant + '.apl')
        path.write_text(text)
        rows.append(dict(variant=variant, source=str(path), source_sha256=sha(path), loc=loc(text)))
    (HERE / 'preserved.json').write_text(json.dumps(dict(pins=preserved, sources=rows), indent=2) + '\n')
    print(json.dumps(rows, indent=2))


if __name__ == '__main__':
    main()
