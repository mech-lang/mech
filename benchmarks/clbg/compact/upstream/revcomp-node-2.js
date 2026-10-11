/*  The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

    contributed by Joe Farro
    parts taken from solution contributed by
    Jos Hirth which was modified by 10iii
*/


const stdout = process.stdout;
const stdin = process.stdin;

const READ_SIZE = 16000;
const writeBuffer = Buffer.allocUnsafe(READ_SIZE + READ_SIZE / 61 | 0);
let metaI;
let numLines;


function LinkedArray(prev) {
    this.prev = prev;
    this.next = undefined;
    this.pos = 0;
    this.data = [];
}


function getCompChar(code) {
    switch (code) {
        case 65:
        case 97: return 84;
        case 66:
        case 98: return 86;
        case 67:
        case 99: return 71;
        case 68:
        case 100: return 72;
        case 71:
        case 103: return 67;
        case 72:
        case 104: return 68;
        case 75:
        case 107: return 77;
        case 77:
        case 109: return 75;
        case 78:
        case 110: return 78;
        case 82:
        case 114: return 89;
        case 83:
        case 115: return 83;
        case 84:
        case 85: return 65;
        case 86:
        case 118: return 66;
        case 87:
        case 119: return 87;
        case 116:
        case 117: return 65;
        case 89:
        case 121: return 82;
    }
}


function reverseCompPrint(line) {
    let _metaI = metaI;
    let _numLines = numLines;
    let count = 0;
    const target = writeBuffer;
    const len = line.length;
    const right = line.length - 1;

    let ileft = 0;
    let iright = 0;

    let c;
    while (iright < len) {
        c = line[right - iright];
        iright++;
        if (c === 10) {
            // skip linebreaks
            if (iright === len) {
                break;
            }
            c = line[right - iright];
            iright++;
        }
        target[ileft] = getCompChar(c);
        ileft++;
        count++;
        if ((count + _metaI - _numLines) % 60 === 0) {
            // need a linebreak
            target[ileft] = 10;
            ileft++;
            count++;
            _numLines++;
        }
    }
    metaI = _metaI + count;
    numLines = _numLines;
    stdout.write(target.slice(0, count).toString('ascii'));
}


function reverse(la_) {
    // reset the metaI and numLines in this section
    metaI = 0;
    numLines = 0;

    var la = la_,
        lines = la.data,
        lnIdx = la.pos - 1,
        line = lines[lnIdx];

    while (true) {
        reverseCompPrint(line);
        lnIdx--;
        line = lines[lnIdx];
        if (line) {
            continue;
        }
        la = la.prev;
        if (la === undefined) {
            break;
        }
        lines = la.data;
        lnIdx = la.pos;
        lnIdx--;
        line = lines[lnIdx];
    }
    if ((metaI - numLines) % 60 !== 0) {
        stdout.write('\n');
    }
}


const LA_LEN = 30;
const headLA = new LinkedArray();
let la = headLA;
let lnIdx = 0;
let lines = la.data;

let needHeader = true;
let headerPartial = '';
let isFirst = true;


function read() {
    let chunk = stdin.read(READ_SIZE);
    let isFinal = false;
    if (!chunk) {
        if (isFirst) {
            isFirst = false;
            return;
        }
        la.pos = lnIdx;
        reverse(la);
        return;
    } else if (chunk.length < READ_SIZE) {
        isFinal = true;
    }
    while (chunk) {
        while (true) {
            // if have read a partial header line, read the rest of it
            if (needHeader) {
                const headerEnds = chunk.indexOf('\n');
                console.log(headerPartial.toString('ascii') + chunk.slice(0, headerEnds).toString('ascii'));
                headerPartial = '';
                chunk = chunk.slice(headerEnds);
                needHeader = false;
            }
            const caretIdx = chunk.indexOf('>');
            if (caretIdx > -1) {
                // there is a caret in this chunk -- process the first part of
                // the chunk and then continue the `while (true)` loop to process
                // the next part of the chunk
                lines[lnIdx] = chunk.slice(0, caretIdx);
                // set chunk to the next section
                chunk = chunk.slice(caretIdx);
                la.pos = lnIdx + 1;
                reverse(la);
                // reset the data holders
                la = headLA;
                lines = la.data;
                la.pos = 0;
                lnIdx = 0;
                // check to see if the entire header line is here
                const headerEnds = chunk.indexOf('\n');
                if (headerEnds < 0) {
                    needHeader = true;
                    headerPartial = chunk;
                    break;
                } else {
                    // log the header
                    console.log(chunk.slice(0, headerEnds).toString('ascii'));
                    // continue processing the rest of the chunk
                    chunk = chunk.slice(headerEnds);
                }
            } else {
                if (lnIdx === LA_LEN) {
                    la.pos = LA_LEN;
                    la = la.next || (la.next = new LinkedArray(la));
                    lines = la.data;
                    lines[0] = chunk;
                    lnIdx = la.pos = 1;
                } else {
                    lines[lnIdx] = chunk;
                    lnIdx++;
                }
                break;
            }
        }
        chunk = stdin.read(READ_SIZE);
    }
    if (isFinal) {
        la.pos = lnIdx;
        reverse(la);
    }
}


stdin.on('readable', read);
