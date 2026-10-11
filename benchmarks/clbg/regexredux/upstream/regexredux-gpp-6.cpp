// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Markus Lenger
//
// This implementation of regex-redux uses the regular expression library PCRE2
// PCRE2 allows to compile regular expressions into machine code at runtime
// (JIT compile) which makes it extremely fast.
//
// See https://www.pcre.org/current/doc/html/index.html for more info on PCRE2.
// The code is heavily commented to enhance readability for people who don't
// know C++. For those who do the comments might be annoying ;)

#include <algorithm>
#include <boost/noncopyable.hpp>
#include <fstream>
#include <future>
#include <iostream>
#include <iterator>
#include <memory>
#include <sstream>
#include <vector>
#define PCRE2_CODE_UNIT_WIDTH 8
#include <pcre2.h>

// Cast std::string to PCRE2 buffer
inline PCRE2_UCHAR8* pcre2_buffer_cast(std::string& str)
{
    return reinterpret_cast<PCRE2_UCHAR8*>(str.data());
}

// Cast std::string to PCRE2 immutable string
inline PCRE2_SPTR8 pcre2_string_cast(const std::string& str)
{
    return reinterpret_cast<PCRE2_SPTR8>(str.c_str());
}

/*
 * This class wraps JIT-compiled PCRE2 regular expressions and data-structures
 * needed for matching the regular expressions against strings. The class is
 * derived from boost::noncopyable to prevent accidental copying.
 */
class regex : private boost::noncopyable
{
public:
    inline explicit regex(const std::string& regex_str)
    {
        compile_regex(regex_str);
        allocate_match_data();
    }

    inline ~regex()
    {
        pcre2_code_free(_code);
        pcre2_match_data_free(_match_data);
        pcre2_match_context_free(_match_context);
        pcre2_jit_stack_free(_jit_stack);
    }

    // Count matches of this regex within subject
    inline std::size_t count_matches(const std::string& subject) const
    {
        // Definition of a functor for counting occurrences
        struct count_functor
        {
            std::size_t match_cnt = 0;
            /* This method (operator) will be called for every match in the
             subject */
            inline void operator()(PCRE2_SPTR subject,
                PCRE2_SIZE match_begin,
                PCRE2_SIZE match_end)
            {
                // We ignore these arguments
                std::ignore = subject;
                std::ignore = match_begin;
                std::ignore = match_end;
                // And just increase the match_cnt
                match_cnt++;
            }
        };
        count_functor func;
        const PCRE2_SPTR subject_begin = pcre2_string_cast(subject);
        const PCRE2_SPTR subject_end = subject_begin + subject.size();
        // Here func.operator() is called for every match
        for_each_match(subject_begin, subject_end, func);
        return func.match_cnt;
    }

    /* Replace all matches of this regex between "subject_begin" and
      "subject_end" with "replacement" and store the result in the
      result_buffer */
    inline PCRE2_SIZE replace_all(const std::string& replacement,
        const PCRE2_SPTR8 subject_begin,
        const PCRE2_SPTR8 subject_end,
        PCRE2_UCHAR* const result_buffer_begin,
        PCRE2_UCHAR* const result_buffer_end) const
    {
        // Definition of a functor for replacing matches with strings
        struct replace_functor
        {
            PCRE2_UCHAR* _result_buffer_ptr;
            PCRE2_UCHAR* const _result_buffer_end;
            const std::string& _replacement;
            PCRE2_SIZE _replacement_size;

            inline replace_functor(PCRE2_UCHAR* const result_buffer_begin,
                PCRE2_UCHAR* const result_buffer_end,
                const std::string& replacement)
                : _result_buffer_ptr(result_buffer_begin)
                , _result_buffer_end(result_buffer_end)
                , _replacement(replacement)
                , _replacement_size(replacement.size())
            {
            }

            // This operator will be called for every match
            inline void operator()(const PCRE2_SPTR subject_ptr,
                const PCRE2_SIZE match_begin,
                const PCRE2_SIZE match_end)
            {
                PCRE2_UCHAR* const next_result_buffer
                    = _result_buffer_ptr + match_begin + _replacement_size;
                if (next_result_buffer > _result_buffer_end)
                {
                    throw std::runtime_error("Result buffer too small");
                }
                // copy portions that did no match
                std::copy(
                    subject_ptr, subject_ptr + match_begin, _result_buffer_ptr);
                _result_buffer_ptr += match_begin;
                // paste replacement string
                std::copy(_replacement.begin(), _replacement.end(),
                    _result_buffer_ptr);
                _result_buffer_ptr = next_result_buffer;
            }

            // Copy characters into the result buffer
            inline PCRE2_UCHAR* copy_into_result_buffer(
                const PCRE2_SPTR begin, const PCRE2_SPTR end)
            {
                // Copy remainder
                if (begin >= end)
                    return _result_buffer_ptr;
                const PCRE2_SIZE size = end - begin;
                if (_result_buffer_ptr + size > _result_buffer_end)
                {
                    throw std::runtime_error("Result buffer too small");
                }
                std::copy(begin, end, _result_buffer_ptr);
                _result_buffer_ptr += size;
                return _result_buffer_ptr;
            }
        };

        // Create an instance of the replace_functor
        replace_functor func(
            result_buffer_begin, result_buffer_end, replacement);
        /* Apply the func.operator() on every match. subject_ptr points to the
         * location just after the last match
         */
        PCRE2_SPTR subject_ptr
            = for_each_match(subject_begin, subject_end, func);
        // Copy remainder from subject to result
        PCRE2_UCHAR* result_buffer_ptr
            = func.copy_into_result_buffer(subject_ptr, subject_end);
        // Return the size of the result
        return result_buffer_ptr - result_buffer_begin;
    }

    inline std::string replace_all(
        const std::string& replacement, const std::string& subject) const
    {
        std::string result;
        result.resize(subject.size());
        PCRE2_UCHAR* buffer_begin = pcre2_buffer_cast(result);
        PCRE2_SPTR8 pcre2_subject = pcre2_string_cast(subject);
        auto result_size = replace_all(replacement, pcre2_subject,
            pcre2_subject + subject.size(), pcre2_buffer_cast(result),
            buffer_begin + result.size());

        result.resize(result_size);
        return result;
    }

private:
    // Higher order function that allows application of functors to matches
    template <typename FUNCTOR>
    inline PCRE2_SPTR8 for_each_match(
        PCRE2_SPTR subject_begin, PCRE2_SPTR subject_end, FUNCTOR& action) const
    {
        PCRE2_SPTR subject_ptr = subject_begin;
        int status = 0;
        auto ovector = pcre2_get_ovector_pointer(_match_data);
        // offset of begin of match will always be stored in this array-element
        PCRE2_SIZE& begin_offset = ovector[0];
        // offset of end of match will always be stored in this array-element
        PCRE2_SIZE& end_offset = ovector[1];
        while (subject_ptr < subject_end
            && (status = pcre2_jit_match(_code, // JIT compiled regex
                    subject_ptr,
                    subject_end - subject_ptr, // Size of subject
                    0, // Offset into subject
                    0, // Flags
                    _match_data, // Match info is stored here
                    nullptr // Match context (none in our case)
                    ))
                > 0)
        {
            // Call the functor
            action(subject_ptr, begin_offset, end_offset);
            subject_ptr += end_offset;
        }
        require_status_good(status);
        return subject_ptr;
    }

    inline void compile_regex(const std::string& regex_str)
    {
        PCRE2_SIZE error_offset;
        int error_number;
        // Parse and compile regular expression into PCRE2 representation
        _code = (pcre2_compile(pcre2_string_cast(regex_str),
            PCRE2_ZERO_TERMINATED, 0, &error_number, &error_offset, nullptr));
        if (!_code)
        {
            throw_pcre2_error(error_number);
        }
        // Now we transform the internal representation into machine code
        require_status_good(pcre2_jit_compile(_code, PCRE2_JIT_COMPLETE));
    }

    // Allocate PCRE2 objects for applying the regular expression
    // and storing the result
    inline void allocate_match_data()
    {
        _match_context = pcre2_match_context_create(nullptr);
        require_allocation_good(_match_context);

        _match_data = pcre2_match_data_create_from_pattern(_code, nullptr);
        require_allocation_good(_match_data);

        _jit_stack = pcre2_jit_stack_create(32 * 1024, 512 * 1024, nullptr);
        require_allocation_good(_jit_stack);

        pcre2_jit_stack_assign(_match_context, nullptr, _jit_stack);
    }

    // Throw runtime_error with error-message from PCRE2
    inline static void throw_pcre2_error(int status)
    {
        std::string msg;
        msg.resize(1024);
        pcre2_get_error_message(status, pcre2_buffer_cast(msg), msg.size());
        throw std::runtime_error(msg.c_str());
    }

    // Throw exception if pinter is nullptr
    inline static void require_allocation_good(void* ptr)
    {
        if (ptr == nullptr)
        {
            throw std::bad_alloc();
        }
    }

    // Throw an exception if a PCRE2 error occurred
    inline static void require_status_good(int status)
    {
        if (status < 0 && status != PCRE2_ERROR_NOMATCH)
        {
            throw_pcre2_error(status);
        }
    }

    pcre2_code* _code = nullptr;
    pcre2_match_data* _match_data = nullptr;
    pcre2_match_context* _match_context = nullptr;
    pcre2_jit_stack* _jit_stack = nullptr;
};

/// Patterns for counting
const char* const count_regexes[] = { "agggtaaa|tttaccct",
    "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t",
    "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct",
    "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct",
    "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct" };

using regex_replace_spec = std::pair<const char* const, const char* const>;

/// Patterns + replacements for replacement operation
const regex_replace_spec replace_specs[] = { { "tHa[Nt]", "<4>" },
    { "aND|caN|Ha[DS]|WaS", "<3>" }, { "a[NSt]|BY", "<2>" }, { "<[^>]*>", "|" },
    { "\\|[^|][^|]*\\|", "-" } };

// Run asynchronous tasks in separate thread
const auto launch_type = std::launch::async;

// Read all data from input-stream and return as string
inline std::string slurp(std::istream& in)
{
    std::string input_data;
    size_t buffer_size = 1u << 14;
    input_data.resize(buffer_size);
    size_t space_left = buffer_size;
    while (in.good())
    {
        if (!space_left)
        {
            space_left = buffer_size;
            buffer_size *= 2;
            input_data.resize(buffer_size);
        }
        in.read(&input_data[buffer_size - space_left], space_left);
        space_left -= in.gcount();
    }
    input_data.resize(buffer_size - space_left);
    return input_data;
}

using counter_list = std::vector<size_t>;

inline counter_list count_occurrences(const std::string& subject)
{
    counter_list counters;
    std::vector<std::future<size_t>> tasks;
    for (const auto& regex_str : count_regexes)
    {
        tasks.emplace_back(
            // Launch task in separate thread
            std::async(launch_type, [&subject, &regex_str]() -> size_t {
                regex re(regex_str);
                return re.count_matches(subject);
            }));
    }
    counter_list results;
    // Get results from all asychronous tasks and store them in "results"
    std::transform(tasks.begin(), tasks.end(), std::back_inserter(results),
        [](auto& task) { return task.get(); });
    return results;
}

inline std::string replace_patterns(const std::string& subject)
{
    PCRE2_SIZE current_size = subject.size();
    // A heuristic value new size = original_size * 1.1
    const PCRE2_SIZE buffer_size = current_size * 1.1;
    std::string source(subject);
    std::string sink;
    source.resize(buffer_size);
    sink.resize(buffer_size);
    for (auto replace_spec : replace_specs)
    {
        auto re = regex(replace_spec.first);
        PCRE2_SPTR8 pcre2_src = pcre2_string_cast(source);
        PCRE2_UCHAR* pcre2_sink = pcre2_buffer_cast(sink);
        current_size = re.replace_all(replace_spec.second, pcre2_src,
            pcre2_src + current_size, pcre2_sink, pcre2_sink + buffer_size);
        std::swap(source, sink);
    }
    source.resize(current_size);
    return source;
}

int main()
{
    try
    {
        std::string input = slurp(std::cin);
        auto clean_input_regex = regex(R"(>[^\n]*\n|\n)");
        // Remove newlines and comments
        std::string clean_input = clean_input_regex.replace_all("", input);

        // Launch counting of occurrences of patterns in separate thread
        auto count_task
            = std::async(launch_type, count_occurrences, clean_input);

        // Replace patterns with strings
        auto processed_input = replace_patterns(clean_input);
        // Get results from the thread that counted the occurrences of patterns
        auto counters = count_task.get();

        // Print occurrences to stdout
        size_t i = 0;
        for (auto counter : counters)
        {
            std::cout << count_regexes[i++] << " " << counter << "\n";
        }

        // Print string lengths to stdout
        std::cout << "\n"
                  << input.size() << "\n"
                  << clean_input.size() << "\n"
                  << processed_input.size() << std::endl;
        return 0;
    }
    catch (std::exception& e)
    {
        std::cerr << "Exception caught: " << e.what() << std::endl;
    }
    return 1;
}
