// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by SirJosh3917
// loosely based on chapel-3, fsharpcore-6, java-3, gcc-1.

using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

unsafe class Program
{
    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    private static void Main(string[] args)
    {
        mpz_t temp1 = default;
        mpz_t temp2 = default;
        mpz_t accumulator = default;
        mpz_t denominator = default;
        mpz_t numerator = default;

        var n = int.Parse(args[0]);

        // use a buffer before writing to the console
        // repeated writes to the console are expensive in my testing on
        // windows (cmd.exe), at least. it *may* be different for linux
        //
        // create a buffer:
        // 10 digits, 1 tab, 1 colon, length of digits, times the amount of rows.
        // it's not exactly the size of the output, but it's big enough.
        //
        // we also manually handle it because muh nanoseconds.
        char* builder = stackalloc char[(10 + 1 + 1 + args[0].Length) * n];

        // maintain a pointer to the start of the buffer - we'll be writing and
        // incrementing builder as we append characters
        var builderStart = builder;

        GmpBindings.mpz_init(&temp1);
        GmpBindings.mpz_init(&temp2);
        GmpBindings.mpz_init_set_ui(&accumulator, 0);
        GmpBindings.mpz_init_set_ui(&denominator, 1);
        GmpBindings.mpz_init_set_ui(&numerator, 1);

        // body borrowed from the chapel-3 program

        int digit;
        for (int i = 0, k = 0; i < n;)
        {
            do
            {
                do
                {
                    k++;

                    // next_Term() rountine
                    var k2 = k * 2 + 1;
                    GmpBindings.mpz_addmul_ui(&accumulator, &numerator, 2);
                    GmpBindings.mpz_mul_ui(&accumulator, &accumulator, k2);
                    GmpBindings.mpz_mul_ui(&denominator, &denominator, k2);
                    GmpBindings.mpz_mul_ui(&numerator, &numerator, k);
                }
                while (GmpBindings.mpz_cmp(&numerator, &accumulator) > 0);

                digit = ExtractDigit(3, &temp1, &temp2, 
                                &numerator, &accumulator, &denominator);
            }
            while (digit != ExtractDigit(4, &temp1, &temp2, 
                                &numerator, &accumulator, &denominator));

            // digit is a number from 0 to 9, so we can just append a single
            // character here
            *builder++ = (char)('0' + digit);

            // comparison from java-3 program
            if (++i % 10 == 0)
            {
                *builder++ = '\t';
                *builder++ = ':';

                // we pass in int.MaxValue to ignore bounds checking since we
                // know that the buffer is large enough to handle any writes we
                // may make
                i.TryFormat(new Span<char>(builder, int.MaxValue),
                        out var charsWritten);
                builder += charsWritten;

                *builder++ = '\n';
            }

            // eliminate digit
            GmpBindings.mpz_submul_ui(&accumulator, &denominator, digit);
            GmpBindings.mpz_mul_ui(&accumulator, &accumulator, 10);
            GmpBindings.mpz_mul_ui(&numerator, &numerator, 10);
        }

        // we have to use Console.Out instead of Console.WriteLine because
        // Console.Out has a method for printing a ROS
        Console.Out.Write(new ReadOnlySpan<char>(
                builderStart, (int)(builder - builderStart)));

        // a lot of the pidigit implementations have memory leaks, we don't ;)
        GmpBindings.mpz_clear(&temp1);
        GmpBindings.mpz_clear(&temp2);
        GmpBindings.mpz_clear(&accumulator);
        GmpBindings.mpz_clear(&denominator);
        GmpBindings.mpz_clear(&numerator);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining | 
            MethodImplOptions.AggressiveOptimization)]
    private static int ExtractDigit(int nth, mpz_t* temp1, mpz_t* temp2, 
            mpz_t* numerator, mpz_t* accumulator, mpz_t* denominator)
    {
        // comment from gcc-1 source:
        // joggling between tmp1_Pointer and tmp2_Pointer, so GMP won't have to
        // use temp buffers
        GmpBindings.mpz_mul_ui(temp1, numerator, nth);
        GmpBindings.mpz_add(temp2, temp1, accumulator);
        GmpBindings.mpz_tdiv_q(temp1, temp2, denominator);

        return GmpBindings.mpz_get_ui(temp1);
    }
}

// ported from fsharpcore-6
// GMP reference: https://gmplib.org/manual/Integer-Internals.html
[StructLayout(LayoutKind.Sequential)]
internal ref struct mpz_t
{
    private int _alloc;
    private int _size;
    private IntPtr _ptr;
}

internal static unsafe class GmpBindings
{
    // bindings from the Java_GMP_Wrapper, java-3.html and fsharpcore-6

    [DllImport("gmp", EntryPoint = "__gmpz_init")]
    internal extern static void mpz_init(mpz_t* x);

    [DllImport("gmp", EntryPoint = "__gmpz_clear")]
    internal extern static void mpz_clear(mpz_t* x);

    [DllImport("gmp", EntryPoint = "__gmpz_add")]
    internal extern static void mpz_add(
            mpz_t* sum, mpz_t* augend, mpz_t* addend);

    [DllImport("gmp", EntryPoint = "__gmpz_addmul_ui")]
    internal extern static void mpz_addmul_ui(
            mpz_t* sum, mpz_t* multiplier, uint multiplicand);

    [DllImport("gmp", EntryPoint = "__gmpz_cmp")]
    internal extern static int mpz_cmp(
            mpz_t* firstNumber, mpz_t* secondNumber);

    [DllImport("gmp", EntryPoint = "__gmpz_get_ui")]
    internal extern static int mpz_get_ui(mpz_t* number);

    [DllImport("gmp", EntryPoint = "__gmpz_init_set_ui")]
    internal extern static void mpz_init_set_ui(
            mpz_t* number, int newValue);

    [DllImport("gmp", EntryPoint = "__gmpz_mul_ui")]
    internal extern static void mpz_mul_ui(
            mpz_t* product, mpz_t* multiplier, int multiplicand);

    [DllImport("gmp", EntryPoint = "__gmpz_submul_ui")]
    internal extern static void mpz_submul_ui(
            mpz_t* difference, mpz_t* minuend, int subtrahend);

    [DllImport("gmp", EntryPoint = "__gmpz_tdiv_q")]
    internal extern static void mpz_tdiv_q(
            mpz_t* quotient, mpz_t* dividend, mpz_t* divisor);
}
