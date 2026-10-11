// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Martin Jambrek
//
// Based on a C++ translation of Rust #7 with minor changes
// made to the SSE2 intrinsics used to lookup nucleotides from
// randomly generated integers using the prescribed LCG algorithm.

#include <algorithm>
#include <array>
#include <atomic>
#include <cmath>
#include <cstdio>
#include <mutex>
#include <optional>
#include <string_view>
#include <thread>
#include <vector>

#ifdef __SSE2__
#include <emmintrin.h>
#endif

#include <boost/range/adaptor/strided.hpp>
#include <boost/range/irange.hpp>

constexpr int LINE_LENGTH = 60;
constexpr uint32_t IM = 139968;
constexpr int LINES = 1024;
constexpr int BLKLEN = LINE_LENGTH * LINES;

constexpr int NUM_THREADS = 2;

struct SpinLock {
  std::atomic_bool lock_ = false;

  void lock() noexcept {
    for (;;) {
      if (!lock_.exchange(true, std::memory_order_acquire))
        return;
      while (lock_.load(std::memory_order_relaxed)) {
#ifdef __SSE2__
        _mm_pause();
#endif
      }
    }
  }

  void unlock() noexcept { lock_.store(false, std::memory_order_release); }
};

using mutex_t = SpinLock;

struct Nucleotide {
  char letter;
  float prob;
};

#ifdef __SSE2__
inline int horizontal_sum(__m128i v) {
  __m128i hi64 = _mm_unpackhi_epi64(v, v);
  __m128i sum64 = _mm_add_epi32(hi64, v);
  __m128i hi32 = _mm_shufflelo_epi16(sum64, _MM_SHUFFLE(1, 0, 3, 2));
  __m128i sum32 = _mm_add_epi32(sum64, hi32);
  return _mm_cvtsi128_si32(sum32);
}
#endif

template <size_t N>
struct WeightedRandom {
  // Pad size to a multiple of 4
  static constexpr size_t padded_size = ((N + 4 - 1) / 4) * 4;

  constexpr WeightedRandom(const std::array<Nucleotide, N>& c) noexcept {
    size_t i = 0;
    float acc = 0.f;
    for (auto&& [l, p] : c) {
      acc += p;
      cumprob[i] = std::floor(acc * IM);
      letters[i] = l;
      i++;
    }
    std::fill_n(cumprob + i, padded_size - i,
                std::numeric_limits<int32_t>::max());
  }

#ifdef __SSE2__
  constexpr char gen_from_u32(uint32_t prob) const noexcept {
    const auto needle = _mm_set1_epi32(prob);
    auto count = _mm_setzero_si128();

    for (size_t i = 0; i < padded_size; i += 4) {
      auto vp = _mm_load_si128((__m128i*)(cumprob + i));
      count = _mm_sub_epi32(count, _mm_cmplt_epi32(vp, needle));
    }
    return letters[horizontal_sum(count)];
  }
#else
  constexpr char gen_from_u32(uint32_t prob) const noexcept {
    auto res = std::find_if(begin(cumprob), begin(cumprob) + N,
                            [prob](auto p) { return prob <= p; });
    auto idx = std::distance(begin(cumprob), res);
    return letters[idx];
  }
#endif

  alignas(32) uint32_t cumprob[padded_size];
  char letters[padded_size];
};

class RandomLCG {
 public:
  RandomLCG(size_t count, int thread_count)
      : count_(count), thread_count_(thread_count) {}

  void reset(size_t count) {
    next_thread_id_ = 0;
    count_ = count;
  }

  template <typename Itr>
  std::optional<size_t> gen(Itr begin, size_t n, int tid) noexcept {
    std::lock_guard<mutex_t> lock(mtx_);
    if (next_thread_id_ != tid) {
      return std::nullopt;
    }
    next_thread_id_ = (next_thread_id_ + 1) % thread_count_;

    auto to_gen = std::min(n, count_);
    std::generate_n(begin, to_gen,
                    [this] { return (seed_ = (seed_ * 3877 + 29573) % IM); });
    count_ -= to_gen;
    return to_gen;
  }

 private:
  mutex_t mtx_;
  uint32_t seed_ = 42;
  size_t count_;
  int thread_count_;
  int next_thread_id_ = 0;
};

class Writer {
 public:
  Writer(int thread_count) : thread_count_(thread_count) {}

  bool write(const char* data, size_t n, int tid) noexcept {
    std::lock_guard<mutex_t> lock(mtx_);
    if (next_thread_id_ != tid)
      return false;
    next_thread_id_ = (next_thread_id_ + 1) % thread_count_;

    std::fwrite(data, 1, n, stdout);

    return true;
  }

 private:
  mutex_t mtx_;
  int thread_count_;
  int next_thread_id_ = 0;
};

// Based off make_Sequence_Buffer() from C gcc #2.
inline std::vector<char> gen_repeat_buffer(std::string_view seq) {
  size_t n_chars = seq.length() * LINE_LENGTH;
  std::vector<char> buffer(n_chars + n_chars / LINE_LENGTH);
  char* bufferOffset = buffer.data();

  std::vector<char> extended(seq.length() + LINE_LENGTH);
  for (size_t i = 0; i < extended.size(); ++i) {
    extended[i] = seq[i % seq.length()];
  }

  char line[LINE_LENGTH + 1];
  size_t offset = 0;
  while (n_chars > 0) {
    size_t line_Length = std::min<size_t>(n_chars, LINE_LENGTH);
    line[line_Length] = '\n';

    std::copy_n(begin(extended) + offset, line_Length, line);

    offset += line_Length;
    offset -= (offset > seq.length()) * seq.length();

    std::copy_n(line, line_Length + 1, bufferOffset);
    bufferOffset += line_Length + 1;
    n_chars -= line_Length;
  }
  return buffer;
}

// Based off repeat_And_Wrap_String() from C gcc #2.
inline void fasta_repeat(std::string_view seq, size_t n_chars) {
  auto sequence = gen_repeat_buffer(seq);
  size_t outputBytes = n_chars + n_chars / LINE_LENGTH;
  while (outputBytes >= sequence.size()) {
    std::fwrite(sequence.data(), sequence.size(), 1, stdout);
    outputBytes -= sequence.size();
  }
  std::fwrite(sequence.data(), outputBytes, 1, stdout);
  std::fputc('\n', stdout);
}

template <size_t N>
inline void fasta_random(int thread_id,
                         RandomLCG& rng,
                         Writer& writer,
                         const WeightedRandom<N>& wr) {
  alignas(32) std::array<uint32_t, BLKLEN> rng_buf;
  alignas(32) std::array<char, BLKLEN + LINES> out_buf;
  while (1) {
    std::optional<size_t> count;
    do {
      count = rng.gen(begin(rng_buf), std::size(rng_buf), thread_id);
    } while (!count);

    if (*count == 0)
      break;

    auto rng_sub = boost::make_iterator_range_n(begin(rng_buf), *count);

    size_t line_count = 0;
    for (auto begin : boost::irange(size_t(0), rng_sub.size()) |
                          boost::adaptors::strided(LINE_LENGTH)) {
      auto end = std::min(begin + LINE_LENGTH, rng_sub.size());

      for (auto j = begin; j < end; ++j) {
        out_buf[j + line_count] = wr.gen_from_u32(rng_sub[j]);
      }

      out_buf[end + line_count] = '\n';
      line_count++;
    }
    while (
        !writer.write(out_buf.data(), rng_sub.size() + line_count, thread_id))
      ;
  }
}

template <size_t N>
void fasta_random_par(RandomLCG& rng, WeightedRandom<N> wr, int num_threads) {
  Writer writer(num_threads);

  std::vector<std::thread> threads(num_threads);

  for (int i = 0; i < num_threads; ++i) {
    threads[i] = std::thread(fasta_random<N>, i, std::ref(rng),
                             std::ref(writer), std::ref(wr));
  }

  for (auto& t : threads)
    t.join();
}

int main(int argc, char* argv[]) {
  const int n = (argc > 1) ? atoi(argv[1]) : 1000;
  const int num_threads =
      std::min<int>(NUM_THREADS, std::thread::hardware_concurrency());

  constexpr std::string_view alu =
      "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTT"
      "GGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTC"
      "GAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACT"
      "AAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTG"
      "TAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCT"
      "TGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCG"
      "CCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTC"
      "TCAAAAA";
  std::fputs(">ONE Homo sapiens alu\n", stdout);
  fasta_repeat(alu, n * 2);

  constexpr std::array<Nucleotide, 15> iub{{
        {'a', 0.27f}, {'c', 0.12f}, {'g', 0.12f},
        {'t', 0.27f}, {'B', 0.02f}, {'D', 0.02f},
        {'H', 0.02f}, {'K', 0.02f}, {'M', 0.02f},
        {'N', 0.02f}, {'R', 0.02f}, {'S', 0.02f},
        {'V', 0.02f}, {'W', 0.02f}, {'Y', 0.02f}
  }};

  RandomLCG rng(n * 3, num_threads);

  std::fputs(">TWO IUB ambiguity codes\n", stdout);
  fasta_random_par(rng, WeightedRandom(iub), num_threads);

  rng.reset(n * 5);

  constexpr std::array<Nucleotide, 4> homosapiens{{
      {'a', 0.3029549426680f},
      {'c', 0.1979883004921f},
      {'g', 0.1975473066391f},
      {'t', 0.3015094502008f},
  }};

  std::fputs(">THREE Homo sapiens frequency\n", stdout);
  fasta_random_par(rng, WeightedRandom(homosapiens), num_threads);

  return 0;
}
