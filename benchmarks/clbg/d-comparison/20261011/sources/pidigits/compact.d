// Canonical unbounded spigot, using Phobos arbitrary precision integers.
import std;

void main(string[] args)
{
    auto n = to!int(args[1]);
    BigInt num = 1;
    BigInt acc = 0;
    BigInt den = 1;
    int printed;
    for (uint k = 1; printed < n; ++k)
    {
        acc = (acc + num * 2) * (2 * k + 1);
        den *= 2 * k + 1;
        num *= k;
        if (num > acc)
            continue;
        auto digit = (num * 3 + acc) / den;
        auto next = (num * 4 + acc) / den;
        if (digit != next)
            continue;
        write(digit);
        ++printed;
        if (printed % 10 == 0)
            writeln("\t:", printed);
        acc = (acc - den * digit) * 10;
        num *= 10;
    }
    if (n % 10)
        writeln(" ".replicate(10 - n % 10), "\t:", n);
}
