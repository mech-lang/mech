module clbg::fasta;
import libc;
import std::math;

const char[] ALU = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

struct Probability
{
    char symbol;
    double probability;
}

fn void cumulative(Probability* values, int count)
{
    double total = 0.0;
    for (int i = 0; i < count; i++)
    {
        total += values[i].probability;
        values[i].probability = total;
    }
}

fn char random_base(Probability* values, int count, int* seed)
{
    *seed = (*seed * 3877 + 29573) % 139968;
    double draw = (double)*seed / 139968.0;
    for (int i = 0; i < count; i++)
    {
        if (draw < values[i].probability)
        {
            return values[i].symbol;
        }
    }
    return values[count - 1].symbol;
}

fn void emit(char* data, usz count)
{
    if (libc::fwrite(data, 1, count, libc::stdout()) != count)
    {
        libc::exit(2);
    }
}

fn void repeat_sequence(char[] title, long length)
{
    emit(title.ptr, (usz)title.len);
    long offset = 0;
    while (offset < length)
    {
        char[61] line;
        int count = (int)math::min(60L, length - offset);
        for (int i = 0; i < count; i++)
        {
            line[i] = ALU[(offset + i) % ALU.len];
        }
        line[count] = '\n';
        emit(&line[0], (usz)(count + 1));
        offset += count;
    }
}

fn void random_sequence(char[] title, Probability* values, int count, long length, int* seed)
{
    emit(title.ptr, (usz)title.len);
    long offset = 0;
    while (offset < length)
    {
        char[61] line;
        int width = (int)math::min(60L, length - offset);
        for (int i = 0; i < width; i++)
        {
            line[i] = random_base(values, count, seed);
        }
        line[width] = '\n';
        emit(&line[0], (usz)(width + 1));
        offset += width;
    }
}

fn int main(int argc, char** argv)
{
    long n = libc::atoll(argv[1]);
    if (n < 0)
    {
        return 2;
    }
    Probability[15] iub = {
        { 'a', 0.27 }, { 'c', 0.12 }, { 'g', 0.12 }, { 't', 0.27 },
        { 'B', 0.02 }, { 'D', 0.02 }, { 'H', 0.02 }, { 'K', 0.02 },
        { 'M', 0.02 }, { 'N', 0.02 }, { 'R', 0.02 }, { 'S', 0.02 },
        { 'V', 0.02 }, { 'W', 0.02 }, { 'Y', 0.02 }
    };
    Probability[4] homo = {
        { 'a', 0.3029549426680 }, { 'c', 0.1979883004921 },
        { 'g', 0.1975473066391 }, { 't', 0.3015094502008 }
    };
    cumulative(&iub[0], 15);
    cumulative(&homo[0], 4);
    int seed = 42;
    repeat_sequence(">ONE Homo sapiens alu\n", 2 * n);
    random_sequence(">TWO IUB ambiguity codes\n", &iub[0], 15, 3 * n, &seed);
    random_sequence(">THREE Homo sapiens frequency\n", &homo[0], 4, 5 * n, &seed);
    return 0;
}
