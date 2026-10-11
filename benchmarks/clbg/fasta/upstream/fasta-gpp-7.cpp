/* The Computer Language Benchmarks Game
https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

contributed by Sylvester Saguban
inspired from C++ G++ #7 from Rafal Rusin and contributors
- uses more features of C++17 and TS concept's 'auto' function parameters
- more efficient multi-threading, which sadly is the reason why this
 this has more code than other implementations.
- data structure from array of struct to struct of arrays for better
  cache locality and higher chance of vectorization

compile with g++ 7.2
with these flags -std=c++17 -march=native -O3 -msse -msse2 -msse3
*/

#include <array>
#include <cstdio>
#include <thread>
#include <algorithm>
#include <mutex>
#include <condition_variable>
#include <atomic>

struct Config
{
    static constexpr unsigned max_threads = 8;
    static constexpr unsigned min_threads = 1;
    static constexpr unsigned chars_per_line = 60;
    static constexpr unsigned max_proc = 4;
    static constexpr unsigned min_thread_work = (chars_per_line + 1) * 84;
    static constexpr unsigned max_thread_buff = (chars_per_line + 1) * 1024;

    static constexpr unsigned im = 139968, ia = 3877, ic = 29573;
    static constexpr float reciprocal = 1.0f / im;
    static inline FILE* output = stdout;
};

class ThreadMgr
{
    struct Proc {
        std::atomic<bool>   once;
        std::mutex          mtx;
        unsigned            index, index_reset;
    };
    std::atomic<unsigned>       _out_sequence = 0;
    std::mutex                  _gen_mutex, _out_mutex;
    std::condition_variable     _cv;
    Proc                        _proc[ Config::max_proc ];
    unsigned                    _id_ctr = 0, _threads = 1, _proc_ctr = 0;
public:
    struct Context {
        unsigned        index;
        union {
            alignas(16) std::array< int, Config::max_thread_buff > ibuff;
            alignas(16) std::array< char, Config::max_thread_buff > cbuff;
        };
    };

    void SetThreadCount( unsigned count ) {
        _threads = std::clamp( count, Config::min_threads,
                               Config::max_threads );
    }

    inline void SyncOutput( unsigned index, const auto& function ) {
        std::unique_lock lock(_out_mutex);
        _cv.wait( lock, [=]{ return index == _out_sequence; });
        function();
        ++_out_sequence;
        _cv.notify_all();
    }

    auto Make( const std::string_view& text, unsigned size, auto output ) {
        unsigned id = _id_ctr++;
        auto& proc = _proc[ _proc_ctr++ ];
        return [=, &proc]( ThreadMgr::Context& tdata ) {
            if( proc.once.exchange(false) )
                this->SyncOutput( id, [&]{
                std::fwrite( text.data(), 1, text.size(), Config::output );
                output( size, tdata );
            });
        };
    }

    auto Make( const std::string_view& text, unsigned size,
               auto generate, auto convert, auto output ) {
        unsigned work_count = std::clamp( size / _threads,
                                          Config::min_thread_work,
                                          Config::max_thread_buff );
        unsigned id = _id_ctr++;
        unsigned limit = id + (size / work_count) + (size % work_count != 0);
        _id_ctr = limit + 1;
        auto& proc = _proc[ _proc_ctr++ ];
        proc.index_reset = id + 1;

        return [=, &proc]( ThreadMgr::Context& tdata ) {
            if( proc.once.exchange(false) )
                this->SyncOutput( id, [&] {
                std::fwrite( text.data(), 1, text.size(), Config::output );
            });

            unsigned start_index = id + 1;
            while(true) {
                unsigned cvsize;
                {
                    std::scoped_lock lock(_gen_mutex);
                    {
                        std::scoped_lock lock(proc.mtx);
                        if( proc.index > limit )
                            break;
                        tdata.index = proc.index++;
                    }
                    cvsize = std::min( work_count, size -
                                       (((tdata.index - start_index)
                                         * work_count)) );
                    generate( cvsize, tdata );
                }

                unsigned char_count = convert( cvsize, start_index,
                                               work_count, tdata );

                this->SyncOutput( tdata.index, [&] {
                    if( tdata.index == limit &&
                            tdata.cbuff[char_count-1] != '\n' )
                        tdata.cbuff[char_count++] = '\n';
                    output( char_count, tdata );
                });
            }
        };
    }

    void RunSequence( auto... f ){
        static_assert( sizeof...(f) <= Config::max_proc,
                       "function chain is too large, "
                       "increase Config::max_proc." );
        std::thread threads[ Config::max_threads ];

        _out_sequence = 0;
        _proc_ctr = 0;
        for( auto& i : _proc ) {
            i.once = true;
            i.index = i.index_reset;
        }

        for( unsigned i = 0; i < _threads; ++i )
            threads[i] = std::thread( [=]{
                Context tdata = {0};
                (f( tdata ), ...);
            });

        for( unsigned i = 0; i < _threads; ++i )
            threads[i].join();
    }
} gdata;

int main(int argc, char *argv[])
{
    static constexpr char alu[] =
        "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAG"
        "GTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGC"
        "CGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCC"
        "GGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTC"
        "CGTCTCAAAAA";

    static constexpr std::array< std::pair< float, char >, 15> iub = {{
         { 0.27f, 'a' }, { 0.12f, 'c' }, { 0.12f, 'g' }, { 0.27f, 't' },
         { 0.02f, 'B' }, { 0.02f, 'D' }, { 0.02f, 'H' }, { 0.02f, 'K' },
         { 0.02f, 'M' }, { 0.02f, 'N' }, { 0.02f, 'R' }, { 0.02f, 'S' },
         { 0.02f, 'V' }, { 0.02f, 'W' }, { 0.02f, 'Y' }
    }};

    static constexpr std::array< std::pair< float, char >, 4> homosapiens = {{
         { 0.3029549426680f, 'a' },
         { 0.1979883004921f, 'c' },
         { 0.1975473066391f, 'g' },
         { 0.3015094502008f, 't' }
    }};

    static auto make_commulative = []( const auto& p ) {
        struct Ret {
            alignas(16) std::array< float, p.size() >  real;
            alignas(16) std::array< char, p.size() >  ch;
        } ret{{p[0].first}, {p[0].second}};

        for(unsigned i = 1; i < p.size(); ++i ) {
            ret.real[i] = ret.real[i-1] + p[i].first;
            ret.ch[i] = p[i].second;
        }
        return ret;
    };

    static auto get_random = [] {
        static unsigned last = 42;
        return (last = (last * Config::ia + Config::ic) % Config::im);
    };

    static auto convert = []( const auto& data, unsigned size,
            unsigned start_index, unsigned max_work,
            ThreadMgr::Context& tdata ) {
        auto iitr = tdata.ibuff.begin(), iend = iitr + size;
        auto citr = tdata.cbuff.begin();
        auto inner_loop = [&]( auto end ) {
            while( iitr != end ) {
                auto f = *iitr++ * Config::reciprocal;
                auto i = std::find_if( data.real.begin(),
                                       data.real.end(),
                                       [f]( auto t ){ return f <= t; });
                *citr++ = data.ch[ i - data.real.begin() ];
            }
            *citr++ = '\n';
        };

        unsigned aligner_count = Config::chars_per_line -
                ((max_work * (tdata.index - start_index))
                 % Config::chars_per_line);

        if( aligner_count > 0 )
            inner_loop( iitr + aligner_count );

        unsigned odd_count = (iend - iitr) % Config::chars_per_line;
        auto iend_blocks = iend - odd_count;
        while( iitr < iend_blocks )
            inner_loop( iitr + Config::chars_per_line );

        if( odd_count ) {
            inner_loop( iitr + odd_count );
            citr -= 1;
        }
        return citr - tdata.cbuff.begin();
    };

    auto generate = []( unsigned size, ThreadMgr::Context& tdata ) {
        for( auto i = tdata.ibuff.begin(), end = i + size; i != end; ++i )
            *i = get_random();
    };

    auto output = []( unsigned size, ThreadMgr::Context& tdata ) {
        std::fwrite( tdata.cbuff.data(), 1, size, Config::output );
    };

    auto alu_out = [&]( unsigned size, ThreadMgr::Context& ) {
        static constexpr auto alu_data = [&] {
            constexpr unsigned char_count = (Config::chars_per_line + 1)
                    * (sizeof(alu) - 1);

            std::array< char, char_count > ret{};
            for(unsigned i = 0, j = 0; i < char_count; ++i, ++j) {
                unsigned ul = std::min(i + Config::chars_per_line, char_count);
                for(; i < ul; ++i )
                    ret[i] = alu[ (i - j) % (sizeof(alu) - 1)];
                ret[i] = '\n';
            }
            return ret;
        }();

        constexpr unsigned char_count = Config::chars_per_line *
                (sizeof(alu) - 1);
        for(; size >= char_count; size -= char_count )
            std::fwrite( alu_data.data(), 1, alu_data.size(), Config::output );
        std::fwrite( alu_data.data(), 1, size +
                     (size / Config::chars_per_line), Config::output );
        std::putc('\n', Config::output );
    };

    auto iub_convert = []( unsigned size, unsigned start_index,
            unsigned max_work, ThreadMgr::Context& tdata ) {
        static constexpr auto iub_data = make_commulative( iub );
        return convert( iub_data, size, start_index, max_work, tdata  );
    };

    auto homo_convert = []( unsigned size, unsigned start_index,
            unsigned max_work, ThreadMgr::Context& tdata ) {
        static constexpr auto hom_data = make_commulative( homosapiens );
        return convert( hom_data, size, start_index, max_work, tdata  );
    };

    gdata.SetThreadCount( std::thread::hardware_concurrency() );

    unsigned n = std::max( 1000, std::atoi( argv[1] ) );

    const auto alu_proc = gdata.Make( ">ONE Homo sapiens alu\n",
                                      n * 2, alu_out );

    const auto iub_proc = gdata.Make( ">TWO IUB ambiguity codes\n",
                                      n * 3, generate, iub_convert, output );

    const auto homo_proc = gdata.Make( ">THREE Homo sapiens frequency\n",
                                       n * 5, generate, homo_convert, output );

    gdata.RunSequence( alu_proc, iub_proc, homo_proc );
    return 0;
}
