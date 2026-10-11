/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Andrey Filatkin
   based on Go source code by Oleg Mazurov
*/

const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');
const os = require('os');

const fact = [];

function fillFact(n) {
    fact[0] = 1;
    for (let i = 1; i <= n; i++) {
        fact[i] = fact[i - 1] * i;
    }
}

function fannkuch(n) {
    return function ({idxMin, idxMax}) {
        const p = [];
        const pp = [];
        const count = Array(n);

        // first permutation
        for (let i = 0; i < n; i++) {
            p[i] = i;
        }
        let idx = idxMin;
        for (let i = n - 1; i > 0; i--) {
            const d = div(idx, fact[i]);
            count[i] = d;
            idx = idx % fact[i];

            for (let j = 0; j < n; j++) {
                pp[j] = p[j];
            }

            for (let j = 0; j <= i; j++) {
                if (j + d <= i) {
                    p[j] = pp[j + d];
                } else {
                    p[j] = pp[j + d - i - 1];
                }
            }
        }

        let maxFlips = 1;
        let checkSum = 0;

        idx = idxMin;
        for (let sign = true; ; sign = !sign) {
            let first = p[0];
            if (first !== 0) {
                let flips = 1;
                if (p[first] !== 0) {
                    for (let j = 1; j < n; j++) {
                        pp[j] = p[j];
                    }
                    let p0 = first;
                    while (true) {
                        flips++;
                        let i = 1;
                        let j = p0 - 1;
                        while (i < j) {
                            const t = pp[i];
                            pp[i] = pp[j];
                            pp[j] = t;
                            i++;
                            j--;
                        }
                        const t = pp[p0];
                        pp[p0] = p0;
                        p0 = t;
                        if (pp[p0] === 0) {
                            break;
                        }
                    }
                }
                if (maxFlips < flips) {
                    maxFlips = flips;
                }
                if (sign) {
                    checkSum += flips;
                } else {
                    checkSum -= flips;
                }
            }

            idx++;
            if (idx === idxMax) {
                break;
            }

            // next permutation
            if (sign) {
                p[0] = p[1];
                p[1] = first;
            } else {
                const t = p[1];
                p[1] = p[2];
                p[2] = t;
                for (let k = 2; ; k++) {
                    count[k]++;
                    if (count[k] <= k) {
                        break;
                    }
                    count[k] = 0;
                    for (let j = 0; j <= k; j++) {
                        p[j] = p[j + 1];
                    }
                    p[k + 1] = first;
                    first = p[0];
                }
            }
        }

        return {maxFlips, checkSum};
    }
}

function div(val, by) {
    return (val - val % by) / by;
}

if (isMainThread) {
    mainThread(+process.argv[2]);
} else {
    workerThread(workerData);
}

async function mainThread(n) {
    fillFact(n);

    const nchunks = 720;
    let chunkSize = div((fact[n] + nchunks - 1), nchunks);
    chunkSize += chunkSize % 2;

    const tasks = [];
    const len = div((fact[n] + chunkSize - 1), chunkSize);
    for (let i = 0; i < len; i++) {
        const idxMin = chunkSize * i;
        const idxMax = Math.min(fact[n], idxMin + chunkSize);
        tasks.push({idxMin, idxMax});
    }

    let flips = 0;
    let chk = 0;

    await threadReduce(tasks, n, ({maxFlips, checkSum}) => {
        if (flips < maxFlips) {
            flips = maxFlips;
        }
        chk += checkSum;
    });

    console.log(`${chk}\nPfannkuchen(${n}) = ${flips}`);
}

function workerThread(n) {
    fillFact(n);

    runWorker(parentPort, fannkuch(n));
}

function threadReduce(tasks, workerData, reducer) {
    return new Promise(resolve => {
        const size = os.cpus().length;
        const workers = new Set();
        let ind = 0;

        for (let i = 0; i < size; i++) {
            const worker = new Worker(__filename, {workerData});

            worker.on('message', message => {
                const name = message.name;

                if (name === 'result') {
                    reducer(message.data);
                }
                if (name === 'ready' || name === 'result') {
                    if (ind < tasks.length) {
                        const data = tasks[ind];
                        ind++;
                        worker.postMessage({name: 'work', data});
                    } else {
                        worker.postMessage({name: 'exit'});
                    }
                }
            });
            worker.on('exit', () => {
                workers.delete(worker);
                if (workers.size === 0) {
                    resolve();
                }
            });

            workers.add(worker);
        }
    });
}

function runWorker(port, onWork) {
    port.on('message', message => {
        const name = message.name;

        if (name === 'work') {
            port.postMessage({
                name: 'result',
                data: onWork(message.data),
            });
        } else if (name === 'exit') {
            process.exit();
        }
    });
    port.postMessage({name: 'ready'});
}
