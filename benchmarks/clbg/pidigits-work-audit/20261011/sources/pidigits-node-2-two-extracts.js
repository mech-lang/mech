/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Alexander Fyodorov's program by Isaac Gouy
*/

function pad(i, last) {
  var res = i.toString();
  var count;
  count = 10 - res.length;
  while (count > 0) {
    last ? res += ' ' : res = '0' + res;
    count--;
  }
  return res;
}

function calculatePi(arg) {
  let i = 0;
  let ns = 0;

  let k = 0n;
  let k1 = 1n;
  let a = 0n;
  let d = 1n;
  let m = 0n;
  let n = 1n;
  let t = 0n;
  let u = 1n;

  while (true) {
    k += 1n;
    k1 += 2n;
    t = n << 1n;
    n *= k;
    a += t;
    a *= k1;
    d *= k1;

    if (a > n) {    
      m = n * 3n + a;
      t = m / d;
      u = (m + n) / d;

      if (t === u) {
        ns = ns * 10 + Number(t);
        i += 1;

        let last = i >= arg;
        if (i % 10 == 0 || last) {
                
          console.log(pad(ns, last) + '\t:' + i);                   
          ns = 0;
        }

        if (last) break;

        a = (a - d * t) * 10n;
        n = n * 10n;
      }
    }
  }
}


calculatePi( +process.argv[2] || 30 );
