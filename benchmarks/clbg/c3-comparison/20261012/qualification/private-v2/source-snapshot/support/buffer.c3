module clbg::buffer;
import libc;

struct Buffer
{
    char* data;
    usz len;
    usz capacity;
}

fn void reserve(Buffer* value, usz needed)
{
    if (needed <= value.capacity)
    {
        return;
    }
    usz capacity = value.capacity ? value.capacity : 256;
    while (capacity < needed)
    {
        capacity *= 2;
    }
    void* grown = libc::realloc(value.data, capacity);
    if (!grown)
    {
        libc::exit(2);
    }
    value.data = grown;
    value.capacity = capacity;
}

fn void append(Buffer* value, char* data, usz count)
{
    reserve(value, value.len + count);
    if (count)
    {
        libc::memcpy(value.data + value.len, data, count);
    }
    value.len += count;
}

fn void byte(Buffer* value, char item)
{
    reserve(value, value.len + 1);
    value.data[value.len] = item;
    value.len++;
}

fn Buffer read_all()
{
    Buffer result = {};
    reserve(&result, 1);
    char[65536] chunk;
    while (true)
    {
        usz count = libc::fread(&chunk, 1, chunk.len, libc::stdin());
        append(&result, &chunk[0], count);
        if (count < chunk.len)
        {
            if (libc::ferror(libc::stdin()))
            {
                libc::exit(2);
            }
            break;
        }
    }
    return result;
}

fn void output(char* data, usz count)
{
    if (count && libc::fwrite(data, 1, count, libc::stdout()) != count)
    {
        libc::exit(2);
    }
}

fn void clear(Buffer* value)
{
    libc::free(value.data);
    *value = {};
}
