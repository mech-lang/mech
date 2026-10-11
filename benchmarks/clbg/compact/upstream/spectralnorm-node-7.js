/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Ian Osgood
   optimized by Roy Williams
   modified for Node.js by Isaac Gouy
   multi thread by Andrey Filatkin
   sequential by Isaac Gouy
*/

const bytesPerFloat = Float64Array.BYTES_PER_ELEMENT;

function mainThread(n) {
    const sab = new SharedArrayBuffer(3 * bytesPerFloat * n);
    const u = new Float64Array(sab, 0, n).fill(1);
    const v = new Float64Array(sab, bytesPerFloat * n, n);
    const w = new Float64Array(sab, 2 * bytesPerFloat * n, n);

    for (let i = 0; i < 10; i++) {
        atAu(u, v, w);
        atAu(v, u, w);
    }

    let vBv = 0;
    let vv = 0;
    for (let i = 0; i < n; i++) {
        vBv += u[i] * v[i];
        vv += v[i] * v[i];
    }

    const result = Math.sqrt(vBv / vv);

    console.log(result.toFixed(9));

    function atAu(u, v, w) {
        au(u, w);
        atu(w, v);
    }

    function au(u, v) {
        for (let i = 0; i < n; i++) {
            let t = 0;
            for (let j = 0; j < n; j++) {
                t += u[j] / a(i, j);
            }
            v[i] = t;
        }
    }

    function atu(u, v) {
        for (let i = 0; i < n; i++) {
            let t = 0;
            for (let j = 0; j < n; j++) {
                t += u[j] / a(j, i);
            }
            v[i] = t;
        }
    }

    function a(i, j) {
        return ((i + j) * (i + j + 1) >>> 1) + i + 1;
    }
}

mainThread(+process.argv[2]);
