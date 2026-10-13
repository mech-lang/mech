module clbg::regexredux;
import libc;
import clbg::threads;
import clbg::buffer;
import clbg::pcre2;

const char*[9] VARIANTS = {
    "agggtaaa|tttaccct",
    "[cgt]gggtaaa|tttaccc[acg]",
    "a[act]ggtaaa|tttacc[agt]t",
    "ag[act]gtaaa|tttac[agt]ct",
    "agg[act]taaa|ttta[agt]cct",
    "aggg[acg]aaa|ttt[cgt]ccct",
    "agggt[cgt]aa|tt[acg]accct",
    "agggta[cgt]a|t[acg]taccct",
    "agggtaa[cgt]|[acg]ttaccct"
};
const char*[5] PATTERNS = { "tHa[Nt]", "aND|caN|Ha[DS]|WaS", "a[NSt]|BY", "<[^>]*>", "\\|[^|][^|]*\\|" };
const char*[5] REPLACEMENTS = { "<4>", "<3>", "<2>", "|", "-" };

fn void* compile(char* pattern)
{
    int error = 0;
    usz offset = 0;
    void* code = pcre2::compile(pattern, libc::strlen(pattern), 0, &error, &offset, null);
    if (!code)
    {
        libc::exit(2);
    }
    return code;
}

fn long count_matches(char* pattern, buffer::Buffer* subject)
{
    void* code = compile(pattern);
    void* result = pcre2::match_data(code, null);
    if (!result)
    {
        libc::exit(2);
    }
    usz start = 0;
    long count = 0;
    while (start <= subject.len)
    {
        int status = pcre2::match(code, subject.data, subject.len, start, 0, result, null);
        if (status == -1)
        {
            break;
        }
        if (status < 0)
        {
            libc::exit(2);
        }
        usz* offsets = pcre2::offsets(result);
        if (offsets[1] <= start)
        {
            libc::exit(2);
        }
        count++;
        start = offsets[1];
    }
    pcre2::free_match_data(result);
    pcre2::free_pattern(code);
    return count;
}

fn buffer::Buffer replace(char* pattern, char* replacement, buffer::Buffer* subject)
{
    void* code = compile(pattern);
    void* result = pcre2::match_data(code, null);
    if (!result)
    {
        libc::exit(2);
    }
    buffer::Buffer output = {};
    buffer::reserve(&output, 1);
    usz replacement_length = libc::strlen(replacement);
    usz start = 0;
    while (start <= subject.len)
    {
        int status = pcre2::match(code, subject.data, subject.len, start, 0, result, null);
        if (status == -1)
        {
            break;
        }
        if (status < 0)
        {
            libc::exit(2);
        }
        usz* offsets = pcre2::offsets(result);
        if (offsets[1] <= start)
        {
            libc::exit(2);
        }
        buffer::append(&output, subject.data + start, offsets[0] - start);
        buffer::append(&output, replacement, replacement_length);
        start = offsets[1];
    }
    buffer::append(&output, subject.data + start, subject.len - start);
    pcre2::free_match_data(result);
    pcre2::free_pattern(code);
    return output;
}

struct Job
{
    char* pattern;
    buffer::Buffer* subject;
    long count;
}

fn int run(void* argument)
{
    Job* job = argument;
    job.count = count_matches(job.pattern, job.subject);
    return 0;
}

fn int main()
{
    buffer::Buffer input = buffer::read_all();
    buffer::Buffer sequence = replace(">.*\n|\n", "", &input);
    usz clean_length = sequence.len;
    Job[9] jobs;
    threads::Handle[7] workers;
    for (int index = 0; index < 9; index++)
    {
        jobs[index] = { VARIANTS[index], &sequence, 0 };
    }
    for (int index = 0; index < 7; index++)
    {
        threads::start(&workers[index], &run, &jobs[index]);
    }
    run(&jobs[7]);
    run(&jobs[8]);
    for (int index = 0; index < 7; index++)
    {
        threads::join(workers[index]);
    }
    for (int index = 0; index < 9; index++)
    {
        libc::printf("%s %lld\n", VARIANTS[index], jobs[index].count);
    }
    for (int index = 0; index < 5; index++)
    {
        buffer::Buffer next = replace(PATTERNS[index], REPLACEMENTS[index], &sequence);
        buffer::clear(&sequence);
        sequence = next;
    }
    libc::printf("\n%llu\n%llu\n%llu\n", (ulong)input.len, (ulong)clean_length, (ulong)sequence.len);
    buffer::clear(&sequence);
    buffer::clear(&input);
    return 0;
}
