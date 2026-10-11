// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Contributed by Sylvester Saguban
// taken some inspirations from C++ G++ #3 from Branimir Maksimovic
//
// Improvements to algorithm of C++ G++ #3:
// - Only doing incremental update to key instead of
//   recomputing it for every insert to hash table,
//   this aligns it to the other fast implementations.
//   Notably C GCC, Rust #6, Rust #4.
// - Returning the hash object by moving instead of
//   returning by copy.
// - Using std::thread instead of std::async so routines
//   are guaranteed to run on their own threads.

// Improvements aimed at better compiler optimizations:
// - Passing the count/string length as a function
//   template argument so it is known in advance by
//   the compiler. For programming languages without
//   value generics, the same optimization can done
//   by making those values as constants.
// - 'Key' class uses this template value so 'size'
//   member variable is not needed inside the class,
//   this also reduced the memory usage against the
//   original implementation.

// compile with these flags:
// -std=c++17 -march=native -msse -msse2 -msse3 -O3

#include <iostream>
#include <iomanip>
#include <cstdint>
#include <string>
#include <algorithm>
#include <map>
#include <thread>
#include <type_traits>
#include <cstring>
#include <vector>
#include <cassert>
#include <ext/pb_ds/assoc_container.hpp>

struct Cfg {
    static constexpr size_t thread_count = 4;
    static constexpr unsigned to_char[4] = {'A', 'C', 'T', 'G'};
    static inline unsigned char to_num[128];
    using Data = std::vector<unsigned char>;

    Cfg() {
        to_num['A'] = to_num['a'] = 0;
        to_num['C'] = to_num['c'] = 1;
        to_num['T'] = to_num['t'] = 2;
        to_num['G'] = to_num['g'] = 3;
    }
} const cfg;

template <size_t size>
struct Key
{
    // select type to use for 'data', if hash key can fit on 32-bit integer
    // then use uint32_t else use uint64_t.
    using Data = typename std::conditional<size<=16, uint32_t, uint64_t>::type;

    struct Hash {
        Data operator()(const Key& t)const{ return t._data; }
    };

    Key(Data data) : _data(data) {
    }

    // uses std::string_view instead of std::string because std::string always
    // allocates a copy from the heap. while std::string_view is only a wrapper
    // of a pointer and a size
    Key(const std::string_view& str) {
        _data = 0;
        for(unsigned i = 0; i < size; ++i){
            _data <<= 2;
            _data |= cfg.to_num[unsigned(str[i])];
        }
    }

    // initialize hash from input data
    void InitKey(const unsigned char * data){
        for(unsigned i = 0; i < size; ++i){
            _data <<= 2;
            _data |= data[i];
        }
    }

    // updates the key with 1 byte
    void UpdateKey(const unsigned char data){
        _data <<= 2;
        _data |= data;
    }

    // masks out excess information
    void MaskKey(){
        _data &= _mask;
    }

    // implicit casting operator to string
    operator std::string() const {
        std::string tmp;
        Data data = _data;
        for(size_t i = 0; i != size; ++i, data >>= 2)
            tmp += cfg.to_char[data & 3ull];
        std::reverse(tmp.begin(), tmp.end());
        return std::move(tmp);
    }

    bool operator== (const Key& in) const {
        return _data == in._data;
    }
private:
    static constexpr Data _mask = ~(Data(-1) << (2 * size));
    Data _data;
};

template <size_t size, typename K = Key<size> >
using HashTable = __gnu_pbds::cc_hash_table<K, unsigned, typename K::Hash>;

template <size_t size>
void Calculate(const Cfg::Data& input, size_t begin, HashTable<size>& table)
{
    // original implementation fully recomputes the hash key for each
    // insert to the hash table. This implementation only partially
    // updates the hash, this is the same with C GCC, Rust #6 and Rust #4
    Key<size> key(0);
    // initialize key
    key.InitKey(input.data() + begin);
    // use key to increment value
    ++table[key];

    auto itr_begin = input.data() + begin + cfg.thread_count;
    auto itr_end = (input.data() + input.size() + 1) - size;
    for(;itr_begin < itr_end; itr_begin += cfg.thread_count) {
        // update the key 1 byte at a time
        constexpr size_t nsize = std::min(size, cfg.thread_count);
        for(unsigned i = 0; i < nsize; ++i)
            key.UpdateKey( itr_begin[i] );
        // then finally mask out excess information
        key.MaskKey();
        // then use key to increment value
        ++table[key];
    }
}

template <size_t size>
auto CalculateInThreads(const Cfg::Data& input)
{
    HashTable<size> hash_tables[cfg.thread_count];
    std::thread threads[cfg.thread_count];

    auto invoke = [&](unsigned begin) {
        Calculate<size>(input, begin, hash_tables[begin]);
    };

    for(unsigned i = 0; i < cfg.thread_count; ++i)
        threads[i] = std::thread(invoke, i);

    for(auto& i : threads)
        i.join();

    auto& frequencies = hash_tables[0];
    for(unsigned i = 1 ; i < cfg.thread_count; ++i)
        for(auto& j : hash_tables[i])
            frequencies[j.first] += j.second;
    // return the 'frequency' by move instead of copy.
    return std::move(frequencies);
}

template <unsigned size>
void WriteFrequencies(const Cfg::Data& input)
{
    // we "receive" the returned object by move instead of copy.
    auto&& frequencies = CalculateInThreads<size>(input);
    std::map<unsigned, std::string, std::greater<unsigned>> freq;
    for(const auto& i: frequencies)
        freq.insert({i.second, i.first});

    const unsigned sum = input.size() + 1 - size;
    for(const auto& i : freq)
        std::cout << i.second << ' ' << (sum ? double(100 * i.first) / sum : 0.0) << '\n';
    std::cout << '\n';
}

template <unsigned size>
void WriteCount( const Cfg::Data& input, const std::string& text ) {
    // we "receive" the returned object by move instead of copy.
    auto&& frequencies = CalculateInThreads<size>(input);
    std::cout << frequencies[Key<size>(text)] << '\t' << text << '\n';
}

int main()
{
    Cfg::Data data;
    std::array<char, 256> buf;

    while(fgets(buf.data(), buf.size(), stdin) && memcmp(">THREE", buf.data(), 6));
    while(fgets(buf.data(), buf.size(), stdin) && buf.front() != '>') {
        if(buf.front() != ';'){
            auto i = std::find(buf.begin(), buf.end(), '\n');
            data.insert(data.end(), buf.begin(), i);
        }
    }
    std::transform(data.begin(), data.end(), data.begin(), [](auto c){
        return cfg.to_num[c];
    });
    std::cout << std::setprecision(3) << std::setiosflags(std::ios::fixed);

    WriteFrequencies<1>(data);
    WriteFrequencies<2>(data);
    // value at left is the length of the passed string.
    WriteCount<3>(data, "GGT");
    WriteCount<4>(data, "GGTA");
    WriteCount<6>(data, "GGTATT");
    WriteCount<12>(data, "GGTATTTTAATT");
    WriteCount<18>(data, "GGTATTTTAATTTATAGT");
}
