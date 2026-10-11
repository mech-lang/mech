/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Denis Gribov
   based on C source code by Ledhug
   js BigInt replaced with gmplib by Andrey Filatkin
   un-buffered by Isaac Gouy
*/

const MPZ = require('mpzjs');
const n = +process.argv[2] || 10000;

let i = 0, ns = 0;
let k = 0;
let k2 = 1;
const acc = MPZ(0);
const den = MPZ(1);
const num = MPZ(1);
const tmp = MPZ();
const d3 = MPZ();
const d4 = MPZ();

while (i < n) {
  k++;
  k2 += 2;

  MPZ.addMul(acc, num, 2);
  MPZ.mul(acc, acc, k2);
  MPZ.mul(den, den, k2);
  MPZ.mul(num, num, k);

  if (num.gt(acc)) {
    continue;
  }

  MPZ.mul(tmp, num, 3);
  MPZ.add(tmp, tmp, acc);
  MPZ.div(d3, tmp, den);

  MPZ.add(tmp, tmp, num);
  MPZ.div(d4, tmp, den);
  
  if (d3.ne(d4)) {
    continue;
  }

  const d = d3.toNumber();
  ns = ns * 10 + d;
  i++;
  let last = i >= n;
  if (i % 10 == 0 || last) {
    console.log(pad(ns, last) + '\t:' + i);    
    ns = 0;
  }

  if (last) break;

  MPZ.subMul(acc, den, d);
  MPZ.mul(acc, acc, 10);
  MPZ.mul(num, num, 10);
}

function pad(i, last) {
  var res = i.toString(), count;
  count = 10 - res.length;
  while (count > 0) {
    last ? res += ' ' : res = '0' + res;
    count--;
  }
  return res;
}
