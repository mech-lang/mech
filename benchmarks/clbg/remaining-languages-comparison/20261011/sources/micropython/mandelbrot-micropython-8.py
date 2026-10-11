# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#   
#     line-by-line from Greg Buchholz's C program



import sys


def main():

    w = h = x = y = bit_num = 0
    byte_acc = 0
    i = 0; iterations = 50
    limit = 2.0
    Zr = Zi = Cr = Ci = Tr = Ti = 0.0

    w = int(sys.argv[1])
    h = w

    sys.stdout.write(f'P4\n{w} {h}\n'); sys.stdout.flush()

    for y in range(h):

        for x in range(w):

            Zr = Zi = 0.0 
            Cr = (2.0 * x / w - 1.5); Ci = (2.0 * y / h - 1.0)        
        
            for i in range(iterations):

                Tr = Zr*Zr - Zi*Zi + Cr
                Ti = 2*Zr*Zi + Ci          
                Zr = Tr; Zi = Ti               
                if Zr*Zr+Zi*Zi > limit*limit:
                    break
            
            
            if Zr*Zr+Zi*Zi > limit*limit: 
                byte_acc = (byte_acc << 1) | 0x00
            else:
                byte_acc = (byte_acc << 1) | 0x01
                
            bit_num += 1         

            if bit_num == 8:
                # Python 2.7 sys.stdout.write(chr(byte_acc))
                sys.stdout.buffer.write(bytes([byte_acc]))        
                byte_acc = 0
                bit_num = 0

            elif x == w - 1:

                byte_acc = byte_acc << (8-w%8)   
                sys.stdout.buffer.write(bytes([byte_acc]))  
                byte_acc = 0
                bit_num = 0

main()
