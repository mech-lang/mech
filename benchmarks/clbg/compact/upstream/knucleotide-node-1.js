/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Contributed by Jesse Millikan
   Modified by Matt Baker
   Ported, modified, and parallelized by Roman Pletnev
*/

'use strict';

var rd = require('readline'), cluster = require('cluster');

function RefNum(num){ this.num = num; }
RefNum.prototype.toString = function() { return this.num.toString(); }

function frequency(seq, length){
    var freq = new Map(), n = seq.length-length+1, key, cur, i = 0;
    for(; i<n; ++i){
        key = seq.substr(i, length);
        cur = freq.get(key);
        cur === undefined ? freq.set(key, new RefNum(1)) : ++cur.num;
    }
    return freq;
}

function sort(seq, length){
    var f = frequency(seq, length), keys = Array.from(f.keys()),
        n = seq.length-length+1, res = '';
    keys.sort((a, b)=>f.get(b)-f.get(a));
    for (var key of keys) res += key+' '+(f.get(key)*100/n).toFixed(3)+'\n';
    res += '\n';
    return res;
}

function find(seq, s){
    var f = frequency(seq, s.length);
    return (f.get(s) || 0)+"\t"+s+'\n';
}

function master() {
    var workers = [], reading = false, seq = '', maxBuf = 2048;

    for (var i=0; i<4;++i) workers.push(cluster.fork({workerId: i}));

    var flush = function(close) {
        for (var w of workers) w.send(seq);
        if (close) for (var w of workers) w.send('eof');
        seq = '';
    };
    var lineHandler = function(line){
        if (reading) {
            if (line[0] !== '>' &&
                    (seq += line.toUpperCase()).length >= maxBuf) flush();
        } else reading = line.substr(0, 6) === '>THREE';
    };
    rd.createInterface(process.stdin, process.stdout)
        .on('line', lineHandler).on('close', function() { flush(true); });

    var jobs = workers.length, results = [];
    var messageHandler = function(i){
        return function(message){
            results[i] = message;
            if (!(--jobs)) {
                process.stdout.write(results.join(''));
                process.exit(0);
            }
        };
    };
    for (var i=0; i<workers.length; ++i)
        workers[i].on('message', messageHandler(i));
}

function worker(){
    var seq = '';
    process.on('message', function(message){
        if (message === 'eof') {
            var res = '';
            switch (process.env.workerId) {
                case '0':
                    res += sort(seq, 1);
                    res += sort(seq, 2);
                    res += find(seq, "GGT");
                    break;
                case '1':
                    res += find(seq, "GGTA");
                    res += find(seq, "GGTATT");
                    break;
                case '2':
                    res += find(seq, "GGTATTTTAATT");
                    break;
                case '3':
                    res += find(seq, "GGTATTTTAATTTATAGT");
                    break;
            }
            process.send(res);
            process.exit();
        } else seq += message;
    });
}

cluster.isMaster ? master() : worker();
