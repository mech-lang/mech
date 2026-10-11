/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Roman Pletnev
   based on C/Dart programs by Petr Prokhorenkov/Jos Hirth et al.
   multi thread by Andrey Filatkin
 */

const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

const LINE_LEN = 60;
const RANDOM_BUF_SIZE = LINE_LEN * 1000;
const OUT_BUF_SIZE = (LINE_LEN + 1) * 1000;
const OUT_SIZE = 128 * 1024;
const NEW_LINE = 10;
const ENCODING = 'binary';

const alu =
    'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTG' +
    'GGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGA' +
    'GACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAA' +
    'AATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAAT' +
    'CCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAAC' +
    'CCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTG' +
    'CACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA';

const ac = [
    {s: 'a', p: 0.27}, {s: 'c', p: 0.12}, {s: 'g', p: 0.12},
    {s: 't', p: 0.27}, {s: 'B', p: 0.02}, {s: 'D', p: 0.02},
    {s: 'H', p: 0.02}, {s: 'K', p: 0.02}, {s: 'M', p: 0.02},
    {s: 'N', p: 0.02}, {s: 'R', p: 0.02}, {s: 'S', p: 0.02},
    {s: 'V', p: 0.02}, {s: 'W', p: 0.02}, {s: 'Y', p: 0.02}
];

const hs = [
    {s: 'a', p: 0.3029549426680}, {s: 'c', p: 0.1979883004921},
    {s: 'g', p: 0.1975473066391}, {s: 't', p: 0.3015094502008}
];


function Out() {
    this.bufferSize = OUT_SIZE;
    this.buf = Buffer.allocUnsafe(this.bufferSize);
    this.ct = 0;
}

Out.prototype.writeString = function(str) {
    this.buf.write(str, this.ct, str.length, ENCODING);
    this.ct += str.length;
};

Out.prototype.writeBuffer = function(buffer, pos, len) {
    if (this.ct + len >= this.bufferSize) {
        this.flush();
    }
    buffer.copy(this.buf, this.ct, pos, pos + len);
    this.ct += len;
};

Out.prototype.flush = function() {
    process.stdout.write(this.buf.subarray(0, this.ct));
    this.ct = 0;
};


if (isMainThread) {
    mainThread();
} else {
    workerThread(workerData);
}

async function mainThread() {
    const n = +process.argv[2];
    const out = new Out();

    out.writeString('>ONE Homo sapiens alu\n');
    repeatFasta(alu, n * 2, out);

    out.writeString('>TWO IUB ambiguity codes\n');
    await randomFasta('ac', n * 3, out);

    out.writeString('>THREE Homo sapiens frequency\n');
    await randomFasta('hs', n * 5, out);

    out.flush();
}

function randomFasta(geneType, count, out) {
    return new Promise(resolve => {
        const cpus = 4;
        const workers = new Map();

        const results = new Map();
        let lastResult = 0;

        let lastInd = 0;

        for (let i = 0; i < cpus; i++) {
            const worker = new Worker(__filename, {workerData: {geneType}});
            const input = new SharedArrayBuffer(RANDOM_BUF_SIZE * Float64Array.BYTES_PER_ELEMENT);
            const output = new SharedArrayBuffer(OUT_BUF_SIZE * Uint8Array.BYTES_PER_ELEMENT);

            workers.set(worker, {input, output, len: 0});
            worker.postMessage({name: 'sab', data: {input, output}});

            worker.on('message', message => {
                const name = message.name;

                if (name === 'ready') {
                    nextChunk(worker);
                } else if (name === 'result') {
                    const {len, chunkInd} = message.data;
                    workers.get(worker).len = len;
                    results.set(chunkInd, worker);
                    processResults();
                }
            });
            worker.on('exit', () => {
                workers.delete(worker);
                if (workers.size === 0) {
                    resolve();
                }
            });
        }

        function nextChunk(worker) {
            if (count > 0) {
                const input = workers.get(worker).input;
                const chunk = Math.min(count, RANDOM_BUF_SIZE);

                generateRandom(input, chunk);
                worker.postMessage({name: 'work', data: {count: chunk, chunkInd: lastInd}});
                count -= chunk;
                lastInd++;
            } else {
                worker.postMessage({name: 'exit'});
            }
        }

        function processResults() {
            while (results.has(lastResult)) {
                const worker = results.get(lastResult);
                const {output, len} = workers.get(worker);
                results.delete(lastResult);

                out.writeBuffer(Buffer.from(output), 0, len);
                nextChunk(worker);
                lastResult++;
            }
        }
    });
}

function workerThread({geneType}) {
    const geneList = makeCumulative(geneType === 'ac' ? ac : hs);
    let input;
    let output;

    parentPort.on('message', message => {
        const name = message.name;

        if (name === 'sab') {
            input = new Float64Array(message.data.input);
            output = new Uint8Array(message.data.output);
            parentPort.postMessage({name: 'ready'});
        } else if (name === 'work') {
            const {count, chunkInd} = message.data;

            const len = generateDna(geneList, input, output, count);
            parentPort.postMessage({name: 'result', data: {len, chunkInd}});
        } else if (name === 'exit') {
            process.exit();
        }
    });
}

function repeatFasta(alu, count, out) {
    const buffer = Buffer.from(alu + alu.substr(0, LINE_LEN), ENCODING);
    const len = alu.length;
    let pos = 0;
    while (count > 0) {
        const line = Math.min(count, LINE_LEN);

        out.writeBuffer(buffer, pos, line);
        out.buf[out.ct++] = NEW_LINE;

        pos += line;
        if (pos > len) {
            pos -= len;
        }
        count -= line;
    }
}

function makeCumulative(ac) {
    const result = [];
    let p = 0;
    for (let i = 0; i < ac.length; ++i) {
        p += ac[i].p;
        result.push({
            c: p,
            sc: ac[i].s.charCodeAt(0)
        });
    }
    return result;
}

let last = 42;

function generateRandom(array, count) {
    const buf = new Float64Array(array);

    let l = last;
    for (let i = 0; i < count; i++) {
        l = (l * 3877 + 29573) % 139968;
        buf[i] = l / 139968;
    }
    last = l;
}

function generateDna(geneList, input, output, count) {
    let i = 0;
    let o = 0;
    while (count > 0) {
        const line = Math.min(count, LINE_LEN);

        for (let j = 0; j < line; j++) {
            output[o++] = randomCode(input[i++]);
        }
        output[o++] = NEW_LINE;

        count -= line;
    }
    return o;

    function randomCode(rand) {
        for (let j = 0; j < geneList.length; j++) {
            if (rand < geneList[j].c) {
                return geneList[j].sc;
            }
        }
        return geneList[geneList.length - 1].sc;
    }
}
