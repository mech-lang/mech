// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
﻿// contributed by Michael Ganss
// derived from PHP version that was
// contributed by Oleksii Prudkyi
// port from pidigits.lua-5.lua (Mike Pall, Wim Couwenberg)
// modified by Craig Russell
// GmpInteger interop class from pidigits-csharpcore-3.cs by Miguel de Icaza

using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;

namespace PiDigits
{
    class Program
    {
        const int DigitsPerLine = 10;

        static void Main(string[] args)
        {
            var n = int.Parse(args[0]);
            var i = 1;

            foreach (var d in GenDigits().Take(n))
            {
                Console.Out.Write(d);
                if ((i % DigitsPerLine) == 0)
                    Console.Out.WriteLine("\t:" + i);
                i++;
            }

            // Pad out any trailing digits for the final line
            if ((n % DigitsPerLine) > 0)
                Console.Out.WriteLine(new string(' ', (DigitsPerLine - (n % DigitsPerLine))) + "\t:" + n);
        }

        private static IEnumerable<int> GenDigits()
        {
            var k = 1;
            var n1 = new GmpInteger(4);
            var n2 = new GmpInteger(3);
            var d = new GmpInteger(1);
            var u = new GmpInteger();
            var v = new GmpInteger();
            var w = new GmpInteger(0);

            while (true)
            {
                // digit
                u.Div(n1, d);
                v.Div(n2, d);

                if (u.Cmp(v) == 0)
                {
                    yield return u.IntValue();

                    // extract
                    u.Mul(u, -10);
                    u.Mul(u, d);
                    n1.Mul(n1, 10);
                    n1.Add(n1, u);
                    n2.Mul(n2, 10);
                    n2.Add(n2, u);
                }
                else
                {
                    // produce
                    var k2 = k * 2;
                    u.Mul(n1, k2 - 1);
                    v.Add(n2, n2);
                    w.Mul(n1, k - 1);
                    n1.Add(u, v);
                    u.Mul(n2, k + 2);
                    n2.Add(w, u);
                    d.Mul(d, k2 + 1);
                    k++;
                }
            }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    struct mpz_t
    {
        public int _mp_alloc;
        public int _mp_size;
        public IntPtr ptr;
    }

    class GmpInteger
    {
        public GmpInteger()
        {
            mpz_init(ref pointer);
        }

        public GmpInteger(int value)
        {
            mpz_init(ref pointer);
            mpz_set_si(ref pointer, value);
        }

        public void Set(int value) => mpz_set_si(ref pointer, value);

        public void Mul(GmpInteger op1, GmpInteger op2) =>
            mpz_mul(ref pointer, ref op1.pointer, ref op2.pointer);

        public void Mul(GmpInteger src, int val) =>
            mpz_mul_si(ref pointer, ref src.pointer, val);

        public void Add(GmpInteger op1, GmpInteger op2) =>
            mpz_add(ref pointer, ref op1.pointer, ref op2.pointer);

        public void Sub(GmpInteger op1, GmpInteger op2) =>
            mpz_sub(ref pointer, ref op1.pointer, ref op2.pointer);

        public void Div(GmpInteger op1, GmpInteger op2) =>
            mpz_tdiv_q(ref pointer, ref op1.pointer, ref op2.pointer);

        public int IntValue() => mpz_get_si(ref pointer);

        public int Cmp(GmpInteger op1) =>
            mpz_cmp(ref pointer, ref op1.pointer);

        mpz_t pointer;

        [DllImport("gmp", EntryPoint = "__gmpz_init")]
        extern static void mpz_init(ref mpz_t value);

        [DllImport("gmp", EntryPoint = "__gmpz_mul")]
        extern static void mpz_mul(ref mpz_t dest, ref mpz_t op1, ref mpz_t op2);

        [DllImport("gmp", EntryPoint = "__gmpz_mul_si")]
        extern static void mpz_mul_si(ref mpz_t dest, ref mpz_t src, int val);

        [DllImport("gmp", EntryPoint = "__gmpz_add")]
        extern static void mpz_add(ref mpz_t dest, ref mpz_t src, ref mpz_t src2);

        [DllImport("gmp", EntryPoint = "__gmpz_sub")]
        extern static void mpz_sub(ref mpz_t dest, ref mpz_t src, ref mpz_t src2);

        [DllImport("gmp", EntryPoint = "__gmpz_tdiv_q")]
        extern static void mpz_tdiv_q(ref mpz_t dest, ref mpz_t src, ref mpz_t src2);

        [DllImport("gmp", EntryPoint = "__gmpz_set_si")]
        extern static void mpz_set_si(ref mpz_t src, int value);

        [DllImport("gmp", EntryPoint = "__gmpz_get_si")]
        extern static int mpz_get_si(ref mpz_t src);

        [DllImport("gmp", EntryPoint = "__gmpz_cmp")]
        extern static int mpz_cmp(ref mpz_t op1, ref mpz_t op2);
    }
}
