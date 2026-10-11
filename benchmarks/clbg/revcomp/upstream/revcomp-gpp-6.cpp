// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by John Schmerge

#include <cstdint>
#include <cstdio>
#include <iostream>
#include <atomic>
#include <vector>
#include <algorithm>
#include <string>
#include <mutex>
#include <thread>
#include <list>
#include <condition_variable>

constexpr size_t line_length = 60;
constexpr unsigned long expected_chunk_size = (1 << 11);

static constexpr char lookup[] = {
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 0
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 16
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 32
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 48

    ' ', 'T', 'V', 'G', 'H', ' ', ' ', 'C', // @ABCDEFG (64-71)
    'D', ' ', ' ', 'M', ' ', 'K', 'N', ' ', // HIJKLMNO (72-79)
    ' ', ' ', 'Y', 'S', 'A', 'A', 'B', 'W', // PQRSTUVW (80-87)
    ' ', 'R', ' ', ' ', ' ', ' ', ' ', ' ', // XYZ[\]^_ (88-95)
    ' ', 'T', 'V', 'G', 'H', ' ', ' ', 'C', // `abcdefg (96-103)
    'D', ' ', ' ', 'M', ' ', 'K', 'N', ' ', // hijklmno (104-111)
    ' ', ' ', 'Y', 'S', 'A', 'A', 'B', 'W', // pqrstuvw (112-119)
    ' ', 'R', ' ', ' ', ' ', ' ', ' ', ' ', // xyz{|}~  (120-127)

    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 128
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 144
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 160
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 176
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 192
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 208
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 224
    ' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ',' ', // 240
};

const struct complement {
    unsigned char operator () (unsigned char c) const {
        return lookup[static_cast<unsigned>(c)];
    }
} complement_functor{};

struct item {
    std::string name;
    std::string sequence;
};

struct item_block {
    item_block(uint32_t seq)
      : sequence_number(seq)
      , unprocessed_count(0)
      , item_list() { }

    item_block(uint32_t seq, std::vector<item> && items)
      : sequence_number(seq)
      , unprocessed_count(items.size())
      , item_list(std::move(items)) { }

    item_block(const item_block & other) = delete;
    item_block & operator = (const item_block & other) = delete;

    uint32_t sequence_number;
    std::atomic<uint32_t> unprocessed_count;
    std::vector<item> item_list;
};

std::mutex input_lock;
std::condition_variable input_available;
std::list<item_block> input_chunks;

std::mutex output_lock;
std::condition_variable output_available;
std::list<std::vector<item>> output_chunks;

void do_reverse_complement(item & w) {
    std::reverse(w.sequence.begin(), w.sequence.end());
    std::transform(w.sequence.begin(), w.sequence.end(), w.sequence.begin(),
                   complement_functor);

    w.name += '\n';
    std::string s;
    s.reserve(w.sequence.size() + 1 + (w.sequence.size() / line_length));

    for (size_t i = 0; i < w.sequence.size(); i += line_length) {
        s += w.sequence.substr(i, std::min(line_length, w.sequence.size() - i));
        s += '\n';
    }

    w.sequence.swap(s);
}

void process_input(const size_t num_workers, std::istream & f) {
    uint32_t sequence_number = ~0;
    std::string line;
    std::vector<item> items;
    items.reserve(expected_chunk_size);

    while (std::getline(f, line)) {
        ++sequence_number;
        if (! line.empty() && line[0] == '>') {
            if (items.size() == expected_chunk_size) {
                std::unique_lock<std::mutex> ul(input_lock);
                input_chunks.emplace_back(sequence_number, std::move(items));
                input_available.notify_all();
                items.clear();
                items.reserve(expected_chunk_size);
            }

            items.emplace_back();
            items.back().name = std::move(line);
            items.back().sequence.reserve(1024);
        } else {
            items.back().sequence += line;
        }
    }
    std::unique_lock<std::mutex> ul(input_lock);
    input_chunks.emplace_back(sequence_number, std::move(items));
    for (size_t i = 0; i < num_workers; ++i)
        input_chunks.emplace_back(sequence_number + i);
    input_available.notify_all();
}

void worker_main(const size_t id, const size_t num_workers) {
    uint32_t last_sequence = 0;
    size_t chunk_size = 0;

    std::unique_lock<std::mutex> ul(input_lock);

    do {
        input_available.wait(ul, [&] {
            return ( (! input_chunks.empty()) 
                  && last_sequence < input_chunks.front().sequence_number);
        } );
        auto chunk = input_chunks.begin();
        chunk_size = chunk->item_list.size();
        last_sequence = chunk->sequence_number;
        ul.unlock();

        uint32_t items_processed = 0;

        for (size_t i = id; i < chunk_size; i += num_workers)
        {
            do_reverse_complement(chunk->item_list[i]);
            ++items_processed;
        }

        auto old_value = chunk->unprocessed_count.fetch_sub(
                           items_processed, std::memory_order_acq_rel);

        ul.lock();
        if (old_value == items_processed) {
            std::unique_lock<std::mutex> oul(output_lock);
            output_chunks.emplace_back(std::move(chunk->item_list));
            output_available.notify_one();
            oul.unlock();
            input_chunks.erase(chunk);
        }
    } while (chunk_size == expected_chunk_size);
}

void output()
{
    size_t chunk_size = 0;
    std::unique_lock<std::mutex> ul(output_lock);
    do {
        output_available.wait(ul, [&] { return (! output_chunks.empty()); } );
        chunk_size = output_chunks.front().size();
        ul.unlock();

        for (auto & seq : output_chunks.front()) {
            fwrite(seq.name.c_str(), 1, seq.name.size(), stdout);
            fwrite(seq.sequence.c_str(), 1, seq.sequence.size(), stdout);
        }

        ul.lock();
        output_chunks.pop_front();
    } while (chunk_size == expected_chunk_size);
}

template<typename T>
 constexpr T saturate(const T & value, size_t shift = 1) {
    return ( ( shift >= (sizeof(value) << 3) ) ? value :
             saturate(static_cast<T>(value | (value >> shift)), shift << 1) );
}

template <typename T>
 constexpr T next_power_of_two(const T & x) {
    return (1 + saturate(x - 1));
}

int main()
{
    size_t num_workers = next_power_of_two(std::thread::hardware_concurrency());
    std::ios::sync_with_stdio(false);
    std::thread input_thread(process_input, num_workers, std::ref(std::cin));
    std::thread output_thread(output);
    std::vector<std::thread> workers;

    fprintf(stderr, "Num workers = %zu\n", num_workers);
    for (size_t i = 0; i < num_workers; ++i)
        workers.emplace_back(worker_main, i, num_workers);

    for (auto & t : workers) t.join();
    input_thread.join();
    output_thread.join();
}
