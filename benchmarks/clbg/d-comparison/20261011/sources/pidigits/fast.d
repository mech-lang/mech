// Strict D port of Mr Ledrug's Benchmark Game C #1, using GMP arithmetic.
// Both extract(3) and extract(4) are performed; all control flow is in D.
import std.conv : to;
import std.stdio : write, writeln;
import std.array : replicate;

struct Z
{
    int allocated;
    int size;
    ulong* limbs;
}
extern(C) @nogc nothrow
{
    void __gmpz_init(ref Z);
    void __gmpz_clear(ref Z);
    void __gmpz_set_ui(ref Z, ulong);
    void __gmpz_mul_ui(ref Z, ref Z, ulong);
    void __gmpz_add(ref Z, ref Z, ref Z);
    void __gmpz_addmul_ui(ref Z, ref Z, ulong);
    void __gmpz_submul_ui(ref Z, ref Z, ulong);
    void __gmpz_tdiv_q(ref Z, ref Z, ref Z);
    int __gmpz_cmp(ref Z, ref Z);
    ulong __gmpz_get_ui(ref Z);
}

void main(string[] args)
{
    auto n = to!int(args[1]);
    Z num, acc, den, tmp, div;
    foreach (p; [&num, &acc, &den, &tmp, &div])
        __gmpz_init(*p);
    scope (exit) foreach (p; [&num, &acc, &den, &tmp, &div])
        __gmpz_clear(*p);
    __gmpz_set_ui(num, 1);
    __gmpz_set_ui(den, 1);
    ulong extract(ulong x)
    {
        __gmpz_mul_ui(tmp, num, x);
        __gmpz_add(div, tmp, acc);
        __gmpz_tdiv_q(tmp, div, den);
        return __gmpz_get_ui(tmp);
    }
    int printed;
    for (ulong k = 1; printed < n; ++k)
    {
        auto k2 = 2 * k + 1;
        __gmpz_addmul_ui(acc, num, 2);
        __gmpz_mul_ui(acc, acc, k2);
        __gmpz_mul_ui(den, den, k2);
        __gmpz_mul_ui(num, num, k);
        if (__gmpz_cmp(num, acc) > 0)
            continue;
        auto digit = extract(3);
        if (digit != extract(4))
            continue;
        write(digit);
        ++printed;
        if (printed % 10 == 0)
            writeln("\t:", printed);
        __gmpz_submul_ui(acc, den, digit);
        __gmpz_mul_ui(acc, acc, 10);
        __gmpz_mul_ui(num, num, 10);
    }
    if (n % 10)
        writeln(" ".replicate(10 - n % 10), "\t:", n);
}
