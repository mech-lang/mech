/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from bearophile's program
   contributed by Isaac Gouy   
*/

const rl = require("node:readline");

async function seq_lines() {
    const stdin = rl.createInterface({
        input: process.stdin,
    });    
    const lines = [];
    let started = false;    
    for await (const line of stdin) {      
        if (started) {
            if (line.startsWith(">")) break;       
            lines.push(line);
        } else {
            started = line.startsWith(">THREE");        
        }
    }  
    return lines;
}

function base_counts(bases, seq) { 
    counts = new Map();
    size = seq.length + 1 - bases;     
    for (let i = 0; i < size; i++) {   
        const nucleo = seq.substring(i,i+bases); 
        var v; if (v = counts.get(nucleo))        
            counts.set(nucleo,v+1); 
        else
            counts.set(nucleo,1);          
    }
    return counts;    
}

function sorted_freq(bases, seq) {
    size = seq.length + 1 - bases;  
    counts = base_counts(bases, seq); 
    return Array.from(counts)
        .sort((a, b) => b[1] - a[1]) 
        .map((a) => [a[0], 100.0 * a[1] / size]);     
}   

function specific_count(code, seq) {  
    counts = base_counts(code.length,seq);
    return counts.get(code) ? counts.get(code) : 0;        
}   

function write_freq(a) {  
    console.log(`${a[0]} ${a[1].toFixed(3)}`);        
}  

function main(n) {
    seq_lines().then(a => {
        const seq = a.map((s) => s.toUpperCase()).join("");
        
        for (const base of [1,2]) {  
            sorted_freq(base,seq).forEach(write_freq);     
            console.log();             
        }
        
        for (const code of ["GGT", "GGTA", "GGTATT",
                "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]) {  
            console.log(`${specific_count(code,seq)}\t${code}`);           
        }   
               
    } );          
}            

main(+process.argv[2] || 0)
