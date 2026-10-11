/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   line-by-line from Greg Buchholz's C program
*/


class Mandelbrot {

public static void Main(String[] args) {   
 
    int w, h, bit_num = 0;
    int byte_acc = 0;
    int i, iter = 50;
    double  x, y, limit = 2.0;
    double Zr, Zi, Cr, Ci, Tr, Ti; 
    var stdout = new BufferedStream(Console.OpenStandardOutput());
    h = Int32.Parse(args[0]); w = h;
       
    Console.WriteLine("P4\n{0} {1}", w, h); 
    
    for(y=0;y<h;y++) 
    {
        for(x=0;x<w;x++)
        {
            Zr = 0.0; Zi = 0.0; Tr = 0.0; Ti = 0.0;
            Cr = (2.0*x/w - 1.5); Ci=(2.0*y/h - 1.0);
        
            for (i=0;i<iter && (Tr+Ti <= limit*limit);++i)
            {
                Zi = 2.0*Zr*Zi + Ci;
                Zr = Tr - Ti + Cr;
                Tr = Zr * Zr;
                Ti = Zi * Zi;
            }
        
            byte_acc = byte_acc << 1;
            if(Tr+Ti <= limit*limit) byte_acc |= 0x01;            
            
            ++bit_num;

            if(bit_num == 8)
            {
                stdout.WriteByte((byte)byte_acc);
                byte_acc = 0;
                bit_num = 0;
            }
            else if(x == w-1)
            {
                byte_acc = byte_acc << (8-w%8);
                stdout.WriteByte((byte)byte_acc);
                byte_acc = 0;
                bit_num = 0;
            }
        }
    }	
    stdout.Flush();    
}
}
