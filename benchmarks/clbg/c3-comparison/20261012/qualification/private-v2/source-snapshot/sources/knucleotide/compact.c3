module clbg::knucleotide;
import libc;
import std::collections::map;
import std::core::mem::allocators;
import clbg::buffer;

alias Map = map::HashMap{ulong, long};
alias MapIterator = map::HashMapIterator{ulong, long};
alias MapEntry = map::Entry{ulong, long};

const int[7] WIDTHS = { 1, 2, 3, 4, 6, 12, 18 };
const char*[5] QUERIES = { "GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT" };

struct Row
{
    ulong key;
    long count;
}

fn buffer::Buffer third_record()
{
    buffer::Buffer input = buffer::read_all();
    buffer::Buffer sequence = {};
    bool selected = false;
    usz offset = 0;
    while (offset < input.len)
    {
        usz end = offset;
        while (end < input.len && input.data[end] != '\n' && input.data[end] != '\r')
        {
            end++;
        }
        usz content_end = end;
        if (content_end > offset && input.data[offset] == '>')
        {
            if (selected)
            {
                break;
            }
            usz width = content_end - offset;
            selected = width >= 6 && libc::memcmp(input.data + offset, ">THREE", 6) == 0 &&
                (width == 6 || input.data[offset + 6] == ' ' || input.data[offset + 6] == '\t');
        }
        else if (selected && content_end > offset && input.data[offset] != ';')
        {
            for (usz i = offset; i < content_end; i++)
            {
                char value = input.data[i];
                if (value >= 'a' && value <= 'z')
                {
                    value -= 32;
                }
                buffer::byte(&sequence, value);
            }
        }
        offset = end;
        if (offset < input.len && input.data[offset] == '\r')
        {
            offset++;
        }
        if (offset < input.len && input.data[offset] == '\n')
        {
            offset++;
        }
    }
    buffer::clear(&input);
    return sequence;
}

fn ulong code(char value)
{
    switch (value)
    {
        case 'A': return 0;
        case 'C': return 1;
        case 'G': return 2;
        case 'T': return 3;
        default: libc::exit(2);
    }
    return 0;
}

fn void histogram(Map* counts, buffer::Buffer* sequence, int width)
{
    counts.init(&allocators::LIBC_ALLOCATOR);
    if (sequence.len < (usz)width)
    {
        return;
    }
    for (usz start = 0; start <= sequence.len - (usz)width; start++)
    {
        ulong key = 0;
        for (int offset = 0; offset < width; offset++)
        {
            key = (key << 2) | code(sequence.data[start + (usz)offset]);
        }
        long* count = counts.get_or_create_ref(key);
        *count += 1;
    }
}

fn int compare(void* left, void* right)
{
    Row* a = left;
    Row* b = right;
    if (a.count != b.count)
    {
        return a.count > b.count ? -1 : 1;
    }
    return a.key < b.key ? -1 : (a.key > b.key ? 1 : 0);
}

fn void frequency(Map* counts, int width)
{
    sz length = counts.len();
    Row* rows = libc::calloc((usz)(length ? length : 1), Row::size);
    if (!rows)
    {
        libc::exit(2);
    }
    MapIterator entries = counts.iter();
    long total = 0;
    for (sz i = 0; i < length; i++)
    {
        MapEntry entry = entries.get(i);
        rows[i] = { entry.key, entry.value };
        total += entry.value;
    }
    libc::qsort(rows, (usz)length, Row::size, &compare);
    for (sz i = 0; i < length; i++)
    {
        char[3] word;
        ulong key = rows[i].key;
        for (int index = width - 1; index >= 0; index--)
        {
            word[index] = "ACGT"[(int)(key & 3)];
            key >>= 2;
        }
        word[width] = 0;
        libc::printf("%s %.3f\n", &word[0], 100.0 * (double)rows[i].count / (double)total);
    }
    libc::putchar('\n');
    libc::free(rows);
}

fn void output(Map* counts)
{
    frequency(&counts[0], 1);
    frequency(&counts[1], 2);
    for (int index = 0; index < 5; index++)
    {
        ulong key = 0;
        char* query = QUERIES[index];
        for (int i = 0; query[i]; i++)
        {
            key = (key << 2) | code(query[i]);
        }
        long count = counts[index + 2].get(key) ?? 0;
        libc::printf("%lld\t%s\n", count, query);
    }
}

fn int main()
{
    buffer::Buffer sequence = third_record();
    Map[7] counts;
    for (int index = 0; index < 7; index++)
    {
        histogram(&counts[index], &sequence, WIDTHS[index]);
    }
    output(&counts[0]);
    for (int index = 0; index < 7; index++)
    {
        counts[index].free();
    }
    buffer::clear(&sequence);
    return 0;
}
