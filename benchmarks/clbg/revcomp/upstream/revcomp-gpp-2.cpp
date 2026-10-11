// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Adam Kewley

#include <iostream>
#include <string>
#include <vector>

#ifdef SIMD
#include <immintrin.h>
#endif

namespace {
    using std::istream;
    using std::ostream;
    using std::runtime_error;
    using std::string;
    using std::bad_alloc;
    using std::vector;

    constexpr size_t basepairs_in_line = 60;
    constexpr size_t line_len = basepairs_in_line + sizeof('\n');

    // custom vector impl. that has *similar* methods to a
    // `vector<char>`. The reason this is necessary is because the stdlib
    // `vector<char>` implementation requires that `.resize` initializes
    // the newly-allocated content, and that `realloc` cannot be
    // used. Valgrind reports that that is ~10-20 % of application
    // perf. for large inputs.
    class unsafe_vector {
    public:
        unsafe_vector() {
            _buf = (char*)malloc(_capacity);
            if (_buf == nullptr) {
                throw bad_alloc{};
            }
        }

        unsafe_vector(const unsafe_vector& other) = delete;
        unsafe_vector(unsafe_vector&& other) = delete;
        unsafe_vector& operator=(unsafe_vector& other) = delete;
        unsafe_vector& operator=(unsafe_vector&& other) = delete;

        ~unsafe_vector() noexcept {
            free(_buf);
        }

        char* data() {
            return _buf;
        }

        // Resizes the vector to have a size of `count`. This method is
        // UNSAFE because any new vector entries are uninitialized.
        void resize_UNSAFE(size_t count) {
            size_t rem = _capacity - _size;
            if (count > rem) {
                grow(count);
            }
            _size = count;
        }

        size_t size() const {
            return _size;
        }

    private:
        void grow(size_t min_cap) {
            size_t new_cap = _capacity;
            while (new_cap < min_cap) {
                new_cap *= 2;
            }

            char* new_buf = (char*)realloc(_buf, new_cap);
            if (new_buf != nullptr) {
                _capacity = new_cap;
                _buf = new_buf;
            } else {
                // The POSIX definition of `realloc` states that a failed
                // reallocation leaves the supplied pointer untouched, so
                // throw here and let the class's destructor free the
                // untouched ptr (if necessary).
                throw bad_alloc{};
            }
        }

        char* _buf = nullptr;
        size_t _size = 0;
        size_t _capacity = 1024;
    };

    // Returns the complement of a a single basepair character. newline
    // characters are unaffected.
    char complement(char character) {
        // this LUT can be made smaller by ANDing with 0x1f (effectively,
        // drag the table up and paste the newline in), but that adds one
        // more instruction (`and`). Benchmarks show a tiny improvement by
        // just doing a straight lookup into a larger LUT.
        static const char complement_lut[] = {
            // [0-32): non-printables: just ensure that newline gets
            // identity-complemented.
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\n', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',

            // [32-64): not useful here
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',
            '\0', '\0', '\0', '\0',  '\0', '\0', '\0', '\0',

            // [64-96): uppercase chars
            '\0', 'T', 'V', 'G',     'H', '\0', '\0', 'C',
            'D', '\0', '\0', 'M',    '\0', 'K', 'N', '\0',
            '\0', '\0', 'Y', 'S',    'A', 'A', 'B', 'W',
            '\0', 'R', '\0', '\0',   '\0', '\0', '\0', '\0',

            // [96-128]: lowercase chars
            '\0', 'T', 'V', 'G',     'H', '\0', '\0', 'C',
            'D', '\0', '\0', 'M',   '\0', 'K', 'N', '\0',
            '\0', '\0', 'Y', 'S',    'A', 'A', 'B', 'W',
            '\0', 'R', '\0', '\0',   '\0', '\0', '\0', '\0'
        };

        return complement_lut[character];
    }

    // Complement then swap `*a` with `*b`
    void complement_swap(char* a, char* b) {
        char tmp = complement(*a);
        *a = complement(*b);
        *b = tmp;
    }

#ifdef SIMD
    __m128i packed(char c) {
        return _mm_set_epi8(c, c, c, c, c, c, c, c, c, c, c, c, c, c, c, c);
    }

    __m128i reverse_complement_simd(__m128i v) {
        // reverse elements in the registers
        v =  _mm_shuffle_epi8(v, _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15));

        // AND all elements with 0x1f, so that a smaller LUT (< 32 bytes)
        // can be used. This is important with SIMD because, unlike
        // single-char complement (above), SIMD uses 16-byte shuffles. The
        // single-char LUT would require four shuffles, this LUT requires
        // two.
        v = _mm_and_si128(v, packed(0x1f));


        // Lookup for all elements <16
        __m128i lt16_mask = _mm_cmplt_epi8(v, packed(16));
        __m128i lt16_els = _mm_and_si128(v, lt16_mask);
        __m128i lt16_lut = _mm_set_epi8('\0', 'N', 'K', '\0',
                                        'M', '\n', '\0', 'D',
                                        'C', '\0', '\0', 'H',
                                        'G', 'V', 'T', '\0');
        __m128i lt16_vals = _mm_shuffle_epi8(lt16_lut, lt16_els);

        // Lookup for all elements >16
        __m128i g16_els = _mm_sub_epi8(v, packed(16));
        __m128i g16_lut = _mm_set_epi8('\0', '\0', '\0', '\0',
                                       '\0', '\0', 'R', '\0',
                                       'W', 'B', 'A', 'A',
                                       'S', 'Y', '\0', '\0');
        __m128i g16_vals = _mm_shuffle_epi8(g16_lut, g16_els);

        // OR both lookup results
        return _mm_or_si128(lt16_vals, g16_vals);
    }
#endif

    // Reverse-complement a contiguous range, [begin, end), of bps.
    //
    // precondition: [begin, end) can be reverse-complemented without
    // needing to account for newlines etc. (the caller should handle this
    // externally).
    void reverse_complement_bps(char* start, char* end, size_t num_bps) {
#ifdef SIMD
        while (num_bps >= 16) {
            end -= 16;

            __m128i tmp = _mm_lddqu_si128((__m128i*)start);
            _mm_storeu_si128((__m128i*)start, reverse_complement_simd(_mm_lddqu_si128((__m128i*)end)));
            _mm_storeu_si128((__m128i*)end, reverse_complement_simd(tmp));

            num_bps -= 16;
            start += 16;
        }
#else
        // even when not using platform-dependent SIMD, it's still
        // advantageous to unroll the loop. This gives a ~10 % speedup on
        // my laptop (intel i7-8550U)
        while (num_bps >= 16) {
            for (size_t i = 0; i < 16; ++i) {
                complement_swap(start++, --end);
            }

            num_bps -= 16;
        }
#endif

        // portable and works for all sizes of inputs, but is slowest.
        for (size_t i = 0; i < num_bps; ++i) {
            complement_swap(start++, --end);
        }
    }

    struct Sequence {
        string header;  // not incl. starting delim (>)
        unsafe_vector seq;  // basepair lines. all lines terminated by newline
    };

    // Reverse-complements a FASTA sequence. Unformatted basepair (no
    // header) input. All lines apart from the last line contain *exactly*
    // 60 basepairs. The last line can contain <= 60 basepairs, and must
    // have a trailing newline.
    //
    // The reason this alg. is more complicated than necessary for several
    // reasons:
    //
    // - If newlines were stripped from the input while reading the input,
    //   then memory usage would be ~1/60th lower and this step would be
    //   mostly branchless (good). However, writing the output would
    //   require re-adding the newlines into some intermediate output
    //   buffer before writing the output (very bad).
    //
    // - If newlines are not stripped from the input, then they need to be
    //   handled by this step. The easiest way to handle the newlines (<10
    //   LOC) is to have an `if (next_char == '\n') skip;` check on each
    //   iteration (front and back). However, this introduces two compare
    //   + (sometimes) jump operations per basepair, plus the main loop
    //   invariant. It also prevents doing multi-basepair swaps (SIMD,
    //   loop unrolling, etc.), which is where the *real* perf gains are
    //   hiding (20-50 %).
    //
    // - So we want to optimize this alg. for branchless, preferably
    //   multi-basepair, swaps + complements. However, the presence of
    //   trailing newlines means that the input might be non-symmetric
    //   (that is, the data cannot be blindly swapped because the newlines
    //   will end up in an incorrect location in the output).
    void reverse_complement(Sequence& s) {
        char* begin = s.seq.data();
        char* end = s.seq.data() + s.seq.size();

        if (begin == end) {
            return;
        }

        size_t len = end - begin;
        size_t trailer_len = len % line_len;

        // skip end-of-data, so that `end` points to the last newline in
        // the input (i.e. "just past the end of the last basepair")
        end--;

        // optimal case: all lines in the input are exactly `line_len` in
        // length, with no trailing bps. The relative offsets (from
        // begin/end) of newlines in the data are symmetrical. Therefore,
        // The algorithm can just reverse + complement the entire input,
        // apart from the last newline.
        if (trailer_len == 0) {

            size_t num_pairs = len/2;
            reverse_complement_bps(begin, end, num_pairs);

            bool has_middle_bp = (len % 2) > 0;
            if (has_middle_bp) {
                begin[num_pairs] = complement(begin[num_pairs]);
            }

            return;
        }

        // suboptimal case: the last line in the sequence is < `line_len`
        // (it is a "trailing" line). This means that newlines in the
        // input appear at non-symmetrical offsets relative to `begin` and
        // `end`. Because of this, the algorithm has to carefully step
        // over the newlines so that they aren't reversed into an
        // incorrect location in the output.
        size_t trailer_bps = trailer_len > 0 ? trailer_len - 1 : 0;

        size_t rem_bps = basepairs_in_line - trailer_bps;
        size_t rem_bytes = rem_bps + 1;

        size_t num_whole_lines = len / line_len;
        size_t num_steps = num_whole_lines / 2;

        // there are at least two whole lines (+ trailer) per iteration of
        // this loop. This means that we can revcomp the trailer, skip the
        // trailer (+ newline, on the trailer's side), then revcomp the
        // remainder, skip the remainder (+newline, on the starting side)
        // to maintain the loop invariant.
        for (size_t i = 0; i < num_steps; ++i) {
            reverse_complement_bps(begin, end, trailer_bps);
            begin += trailer_bps;
            end -= trailer_len;

            reverse_complement_bps(begin, end, rem_bps);
            begin += rem_bytes;
            end -= rem_bps;
        }

        // there may be one whole line (+ trailer) remaining. In this
        // case, we do the first step of the above (revcomp the trailer)
        // but *not* the second (revcomp the remainder) because the
        // remainder will overlap.
        bool has_unpaired_line = (num_whole_lines % 2) > 0;
        if (has_unpaired_line) {
            reverse_complement_bps(begin, end, trailer_bps);
            begin += trailer_bps;
            end -= trailer_len;
        }

        // no *whole* lines remaining, but there may be bytes remaining on
        // the current line.
        size_t bps_in_last_line = end - begin;
        size_t swaps_in_last_line = bps_in_last_line/2;
        reverse_complement_bps(begin, end, swaps_in_last_line);

        // edge case: there is exactly one byte in the middle of the input
        // that needs to be complemented *but not swapped*.
        bool has_unpaired_byte = (bps_in_last_line % 2) > 0;
        if (has_unpaired_byte) {
            begin[swaps_in_last_line] = complement(begin[swaps_in_last_line]);
        }
    }

    void read_up_to(istream& in, unsafe_vector& out, char delim) {
        constexpr size_t read_size = 1<<16;

        size_t bytes_read = 0;
        out.resize_UNSAFE(read_size);
        while (in) {
            in.getline(out.data() + bytes_read, read_size, delim);
            bytes_read += in.gcount();

            if (in.fail()) {
                // failed because it ran out of buffer space. Expand the
                // buffer and perform another read
                out.resize_UNSAFE(bytes_read + read_size);
                in.clear(in.rdstate() & ~std::ios::failbit);
            } else if (in.eof()) {
                // hit EOF, rather than delmiter, but an EOF can be
                // treated almost identially to a delmiter, except that we
                // don't remove the delimiter from the read buffer.
                break;
            } else {
                // succeeded in reading *up to and including* the sequence
                // delimiter. Remove the delmiter.
                --bytes_read;
                break;
            }
        }
        out.resize_UNSAFE(bytes_read);
    }

    // Read a sequence, starting *after* the first delimiter (>)
    void read_sequence(istream& in, Sequence& out) {
        out.header.resize(0);
        std::getline(in, out.header);
        read_up_to(in, out.seq, '>');
    }

    void write_sequence(ostream& out, Sequence& s) {
        out << '>';
        out << s.header;
        out << '\n';
        out.write(s.seq.data(), s.seq.size());
    }
}

namespace revcomp {
    // Reverse-complement an istream of FASTA data (in) and write the
    // output into `out`.
    void reverse_complement_fasta_stream(istream& in, ostream& out) {
        // the read function assumes that '>' has already been read
        // (because istream::getline will read it per loop iteration:
        // prevents needing to 'peek' a bunch).
        if (in.get() != '>') {
            throw runtime_error{"unexpected input: next char should be the start of a seqence header"};
        }

        Sequence s;
        while (not in.eof()) {
            read_sequence(in, s);
            reverse_complement(s);
            write_sequence(out, s);
        }
    }
}

#ifndef NO_MAIN
int main() {
    // required for *large* (e.g. 1 GiB) inputs
    std::cin.sync_with_stdio(false);
    std::cout.sync_with_stdio(false);

    revcomp::reverse_complement_fasta_stream(std::cin, std::cout);
}
#endif
