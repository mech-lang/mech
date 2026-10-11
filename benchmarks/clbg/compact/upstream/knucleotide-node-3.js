/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Based on Node js #2 implementation of Jesse Millikan, Matt Baker and Roman Pletnev
   Contributed by Dani Biro
*/

'use strict';

const rd = require('readline');
const { Worker, isMainThread, parentPort, workerData } = require('worker_threads');

function RefNum (num) { this.num = num; }
RefNum.prototype.toString = function () { return this.num.toString(); };

function sliceToInt (seq, start, end) {
  let val = 0;
  for (let i = start; i < end; ++i) {
    val = (val * 4) + seq[i];
  }
  return val;
}

function bitwiseTrimLeft (value, length) {
  return (
    value - (Math.floor(value / Math.pow(2, length)) * Math.pow(2, length))
  );
}

const keyMap = ['a', 't', 'c', 'g'];

function keyToString (value, length) {
  const str = new Array(length);
  for (let i = 0; i < length; ++i) {
    const key = bitwiseTrimLeft(value, 2);
    str[length - i - 1] = keyMap[key];
    value = Math.floor(value / 4);
  }
  return str.join('').toUpperCase();
}

function frequency (seq, length) {
  const freq = new Map();
  const n = seq.length - length + 1;
  const maskLength = length * 2 - 2;
  let key = sliceToInt(seq, 0, length);

  for (let i = length; i < n; ++i) {
    const cur = freq.get(key);
    if (cur === undefined) {
      freq.set(key, new RefNum(1));
    } else {
      cur.num++;
    }
    key = bitwiseTrimLeft(key, maskLength) * 4 + seq[i];
  }
  return freq;
}

function sort (seq, length) {
  const f = frequency(seq, length);
  const keys = Array.from(f.keys());
  const n = seq.length - length + 1;
  const res = [];
  keys.sort((a, b) => f.get(b) - f.get(a));
  const len = keys.length;

  for (let i = 0; i < len; ++i) {
    const key = keys[i];
    res.push(`${keyToString(key, length)} ${(f.get(key) * 100 / n).toFixed(3)}\n`);
  }

  res.push('\n');
  return res.join('');
}

function toTransformedBuffer (str) {
  const sharedArray = new Uint8Array(new SharedArrayBuffer(str.length));
  for (let i = 0; i < sharedArray.length; ++i) {
    switch (str.charAt(i)) {
      case 't':
        sharedArray[i] = 1;
        break;
      case 'c':
        sharedArray[i] = 2;
        break;
      case 'g':
        sharedArray[i] = 3;
        break;
    }
  }
  return sharedArray;
}

function find (seq, str) {
  const f = frequency(seq, str.length);
  const buffer = toTransformedBuffer(str);
  return `${f.get(sliceToInt(buffer, 0, str.length)) || 0}\t${str.toUpperCase()}\n`;
}

function master () {
  const results = new Array(4);
  let lines = [];
  let reading = false;
  let jobs = 4;
  let currentLen = 0;
  let processing = false;
  let totalBack = 0;
  const BUFFER_LIMIT = 1024 * 128;
  let buffers = [];

  const finishReading = function () {
    const sharedArray = new Uint8Array(new SharedArrayBuffer(totalBack));
    let index = 0;
    for (let i = 0; i < buffers.length; ++i) {
      sharedArray.set(buffers[i], index);
      index += buffers[i].length;
    }
    processing = true;
    buffers = [];
    lines = [];
    for (let i = 0; i < 4; ++i) {
      workers[i].postMessage({ seq: sharedArray });
    }
  };

  const messageHandler = function (workerId) {
    return function (message) {
      if (!processing) {
        buffers.push(message.buf);
        totalBack += message.buf.length;
        if (message.done) {
          finishReading();
        }
      } else {
        results[workerId] = message;
        if (--jobs === 0) {
          process.stdout.write(results.join(''));
          process.exit(0);
        }
      }
    };
  };

  const workers = [...Array(jobs)].map((_, workerId) => {
    const worker = new Worker(__filename, { workerData: { workerId } });
    worker.on('message', messageHandler(workerId));
    return worker;
  });

  const readOnThread = function (done) {
    const str = lines.join('');
    const sharedArray = new Uint8Array(new SharedArrayBuffer(str.length));
    sharedArray.set(Buffer.from(str, 'ascii'));
    workers[0].postMessage({ buf: sharedArray, done });
  };

  const lineHandler = function (line) {
    if (reading) {
      lines.push(line);
      currentLen += line.length;
      if (currentLen > BUFFER_LIMIT) {
        readOnThread(false);
        lines = [];
        currentLen = 0;
      }
    } else if (line[0] === '>') {
      reading = line.slice(0, 6) === '>THREE';
    };
  };

  rd.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
  })
    .on('line', lineHandler)
    .on('close', function () {
      readOnThread(true);
    });
}

function transformArray (arr) {
  for (let i = 0; i < arr.length; ++i) {
    const x = arr[i];
    if (x === 97) {
      arr[i] = 0;
    } else if (x === 116) {
      arr[i] = 1;
    } else if (x === 99) {
      arr[i] = 2;
    } else {
      arr[i] = 3;
    }
  }
}

function worker () {
  parentPort.on('message', (message) => {
    if (message.buf) {
      transformArray(message.buf);
      parentPort.postMessage(message);
    } else {
      const seq = message.seq;
      const res = [];
      switch (workerData.workerId) {
        case 0:
          res.push(sort(seq, 1));
          res.push(sort(seq, 2));
          res.push(find(seq, 'ggt'));
          break;
        case 1:
          res.push(find(seq, 'ggta'));
          res.push(find(seq, 'ggtatt'));
          break;
        case 2:
          res.push(find(seq, 'ggtattttaatt'));
          break;
        case 3:
          res.push(find(seq, 'ggtattttaatttatagt'));
          break;
      }
      parentPort.postMessage(res.join(''));
    }
  });
}

if (isMainThread) {
  master();
} else {
  worker();
}
