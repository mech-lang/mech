/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   line-by-line from Greg Buchholz's C program 
*/




function main(n)
{
    var w, h, bit_num = 0
    const byte_acc = new Uint8Array(1)
    var i; const iter = 50
    var x, y, limit = 2.0    
    var Zr, Zi, Cr, Ci, Tr, Ti

    h = +process.argv[2] || 200, w = h

    process.stdout.write(`P4\n${w} ${h}\n`)

    for (let y = 0; y < h; ++y) 
    {
        for (let x = 0; x < w; ++x) 
        {
            Zr = Zi = Tr = Ti = 0.0
            Cr = 2.0*x/w - 1.5; Ci = 2.0*y/h - 1.0
            
            for (i= 0; i<iter && (Tr+Ti <= limit*limit); ++i) 
            {
                Zi = 2.0*Zr*Zi + Ci
                Zr = Tr - Ti + Cr
                Tr = Zr * Zr
                Ti = Zi * Zi
            }

            byte_acc[0] <<= 1
            if (Tr+Ti <= limit*limit) byte_acc[0] |= 0x01

            ++bit_num
            
            if (bit_num === 8) 
            {
                process.stdout.write(byte_acc) 
                byte_acc[0] = 0
                bit_num = 0
            } 
            else if (x === w-1) 
            {
                byte_acc[0] <<= (8-w%8)   
                process.stdout.write(byte_acc)                 
                byte_acc[0] = 0
                bit_num = 0
            }
        }
    }
}

main()
