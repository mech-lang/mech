// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Translated from Mr Ledrug's C program by Jeremy Zerfas.

final class pidigits extends Java_GMP_Wrapper{
    // These variables are used to store pointers to the GMP numbers we will be
    // using in this program. These pointers are being stored as Java 64 bit
    // signed longs and this could potentially cause a few problems. One problem
    // could occur if the program is ran on a computer that is using more than
    // 64 bits of address space. The second problem that could possibly occur is
    // that the pointers could be corrupted while being converted to and from
    // Java longs (if the machine architecture and/or C implementation is
    // weird). Neither of these problems is likely to occur on just about any
    // present day computer capable of running Java and if it is a problem you
    // could work around it by storing the pointers in Java ByteBuffers or byte
    // arrays (but those are about 5% slower from my testing and also increase
    // the code size).
    static final long tmp1_Pointer=allocate_mpz_t(),
      tmp2_Pointer=allocate_mpz_t(), acc_Pointer=allocate_mpz_t(),
      den_Pointer=allocate_mpz_t(), num_Pointer=allocate_mpz_t();


    static final long extract_Digit(final long nth){
        // joggling between tmp1_Pointer and tmp2_Pointer, so GMP won't have to
        // use temp buffers
        mpz_mul_ui(tmp1_Pointer, num_Pointer, nth);
        mpz_add(tmp2_Pointer, tmp1_Pointer, acc_Pointer);
        mpz_tdiv_q(tmp1_Pointer, tmp2_Pointer, den_Pointer);

        return mpz_get_ui(tmp1_Pointer);
    }


    static final void eliminate_Digit(final long d){
        mpz_submul_ui(acc_Pointer, den_Pointer, d);
        mpz_mul_ui(acc_Pointer, acc_Pointer, 10);
        mpz_mul_ui(num_Pointer, num_Pointer, 10);
    }


    static final void next_Term(final long k){
        final long k2=k*2+1;
        mpz_addmul_ui(acc_Pointer, num_Pointer, 2);
        mpz_mul_ui(acc_Pointer, acc_Pointer, k2);
        mpz_mul_ui(den_Pointer, den_Pointer, k2);
        mpz_mul_ui(num_Pointer, num_Pointer, k);
    }


    public static final void main(final String[] args){
        final long n=Long.parseLong(args[0]);

        mpz_init_set_ui(tmp1_Pointer, 0);
        mpz_init_set_ui(tmp2_Pointer, 0);

        mpz_init_set_ui(acc_Pointer, 0);
        mpz_init_set_ui(den_Pointer, 1);
        mpz_init_set_ui(num_Pointer, 1);

        // It's considerably faster to use a StringBuilder to buffer all the
        // output and do one single print() call instead of many print(ln)()
        // calls. A BufferedWriter could be used alternatively and works about
        // equally well and is a little more flexible but it also increases the
        // code size too.
        final StringBuilder results=new StringBuilder();

        for(long i=0, k=0; i<n;){
            next_Term(++k);
            if(mpz_cmp(num_Pointer, acc_Pointer)>0)
                continue;

            final long d=extract_Digit(3);
            if(d!=extract_Digit(4))
                continue;

            results.append(d);
            if(++i%10==0)
                results.append("\t:").append(i).append("\n");
            eliminate_Digit(d);
        }

        System.out.print(results);
    }
}


class Java_GMP_Wrapper{
    static {
        System.loadLibrary("Java_GMP_Wrapper");
    }

    // Note that the following *_ui() functions will be calling GMP functions of
    // the same name and they will be expecting C unsigned longs (which are at
    // least 32 bits in size) but we'll actually be passing them Java signed
    // longs (which are 64 bits). All of these functions except for
    // mpz_addmul_ui() and mpz_submul_ui() have a _si vesion that uses signed
    // longs instead but I just decided to use the _ui versions for all of these
    // anyway to be consistent with the original C program this was translated
    // from and since I already had no choice for the mpz_addmul_ui() and
    // mpz_submul_ui() functions. This shouldn't be an issue as long as the
    // machine architecture and/or C implementation isn't weird which is the
    // case for just about every present day computer capable of running Java.
    //
    // Also see the comment up above regarding the pointers.
    final static native long allocate_mpz_t();
    final static native void mpz_add(long sum_Pointer, long augend_Pointer,
      long addend_Pointer);
    final static native void mpz_addmul_ui(long sum_Pointer,
      long multiplier_Pointer, long multiplicand);
    final static native int mpz_cmp(long first_Number_Pointer,
      long second_Number_Pointer);
    final static native long mpz_get_ui(long number_Pointer);
    final static native void mpz_init_set_ui(long number_Pointer,
      long new_Value);
    final static native void mpz_mul_ui(long product_Pointer,
      long multiplier_Pointer, long multiplicand);
    final static native void mpz_submul_ui(long difference_Pointer,
      long minuend_Pointer, long subtrahend);
    final static native void mpz_tdiv_q(long quotient_Pointer,
      long dividend_Pointer, long divisor_Pointer);
}
