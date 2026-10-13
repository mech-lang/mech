module clbg::revcomp;
import libc;
import clbg::buffer;

fn char complement(char base)
{
    if (base >= 'a' && base <= 'z')
    {
        base -= 32;
    }
    switch (base)
    {
        case 'A': return 'T';
        case 'C': return 'G';
        case 'G': return 'C';
        case 'T': return 'A';
        case 'U': return 'A';
        case 'M': return 'K';
        case 'R': return 'Y';
        case 'W': return 'W';
        case 'S': return 'S';
        case 'Y': return 'R';
        case 'K': return 'M';
        case 'V': return 'B';
        case 'H': return 'D';
        case 'D': return 'H';
        case 'B': return 'V';
        case 'N': return 'N';
        default: return base;
    }
}

fn void output_record(buffer::Buffer* sequence)
{
    usz left = 0;
    usz right = sequence.len;
    while (left < right)
    {
        right--;
        char first = complement(sequence.data[left]);
        sequence.data[left] = complement(sequence.data[right]);
        sequence.data[right] = first;
        left++;
    }
    buffer::Buffer output = {};
    usz offset = 0;
    while (offset < sequence.len)
    {
        usz count = sequence.len - offset;
        if (count > 60)
        {
            count = 60;
        }
        buffer::append(&output, sequence.data + offset, count);
        buffer::byte(&output, '\n');
        offset += count;
    }
    buffer::output(output.data, output.len);
    buffer::clear(&output);
    sequence.len = 0;
}

fn int main()
{
    buffer::Buffer input = buffer::read_all();
    buffer::Buffer sequence = {};
    bool record = false;
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
            if (record)
            {
                output_record(&sequence);
            }
            buffer::output(input.data + offset, content_end - offset);
            buffer::output("\n", 1);
            record = true;
        }
        else if (record && content_end > offset && input.data[offset] != ';')
        {
            buffer::append(&sequence, input.data + offset, content_end - offset);
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
    if (record)
    {
        output_record(&sequence);
    }
    buffer::clear(&sequence);
    buffer::clear(&input);
    return 0;
}
