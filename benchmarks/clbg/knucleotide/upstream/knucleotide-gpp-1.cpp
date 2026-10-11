/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Malte Skarupke
*/

#include <iostream>
#include <iomanip>
#include <cstdint>
#include <cstdio>
#include <string>
#include <cstring>
#include <algorithm>
#include <future>
#include <unistd.h>
#include <vector>
#include <ext/pb_ds/assoc_container.hpp>

constexpr const unsigned char tochar[8] =
{
   'A', 'A',
   'C', 'C',
   'T', 'T',
   'G', 'G'
};

// the mask used in the running hash. for a string of size 1
// this is 0b111, for a string of size 2 this is 0b11111,
// for a string of size 4 it's 0b111111111 etc.
// so it sets the lower 2n+1 bits to 1.
//
// this makes it so that the running hash can just keep on shifting
// to the left, and this mask will remove higher bits
uint64_t mask_for_size(unsigned size)
{
   return (1llu << ((size << 1llu) + 1llu)) - 1llu;
}

// incremental hash function that can hash strings of the
// characters 'a', 'A', 'c', 'C', 't', 'T', 'g', 'G'
// it works by realizing that the second and third last bit
// of these are all different. 'a' ends with 0b001,
// 'c' ends with 0b011, 't' ends with 0b100 and 'g'
// ends with 0b111. so we use the mask 0b110 to tell
// them all apart
struct RunningHash
{
   RunningHash()
      : data(0)
   {
   }
   RunningHash(const std::string& s)
      : data(0)
   {
      uint64_t mask = mask_for_size(s.size());
      for (char c : s)
      {
         push(c, mask);
      }
   }
   void push(char c, uint64_t mask)
   {
      // shift old characters over
      data <<= 2;
      // mask out oldest character
      data &= mask;
      // add two bits from the new character
      data |= c & 0b110;
   }
   bool operator<(const RunningHash & in)const
   {
      return data < in.data;
   }
   bool operator==(const RunningHash & in)const
   {
      return data == in.data;
   }
   std::string to_string(unsigned size)const
   {
      std::string tmp;
      tmp.reserve(size);
      uint64_t tmp1 = data;
      for(unsigned i = 0; i != size; ++i)
      {
         tmp += tochar[tmp1 & 0b110];
         tmp1 >>= 2;
      }
      std::reverse(tmp.begin(), tmp.end());
      return tmp;
   }
   struct hash
   {
      uint64_t operator()(const RunningHash & t) const
      {
         return t.data;
      }
   };
   uint64_t data;
};

// comment this next line to run a single-threaded version
#define MULTI_THREADED
#ifdef MULTI_THREADED
std::launch launch_policy = std::launch::async;
#else
std::launch launch_policy = std::launch::deferred;
#endif

typedef __gnu_pbds::cc_hash_table
<
   RunningHash,
   unsigned,
   RunningHash::hash
> HashMap;

HashMap calculate(const char * begin, const char * end, unsigned size)
{
   HashMap frequencies;
   RunningHash tmp;
   uint64_t mask = mask_for_size(size);
   // push the first n-1 characters without adding them to the hash
   // table. need to do this to initialize the incremental hash
   for (const char * init_end = begin + size - 1; begin != init_end; ++begin)
   {
      tmp.push(*begin, mask);
   }
   // add one characer at a time, counting how often each pattern occurs
   for (; begin != end; ++begin)
   {
      tmp.push(*begin, mask);
      ++frequencies[RunningHash(tmp)];
   }
   return frequencies;
}

void write_frequencies(const HashMap & f, unsigned input_size, unsigned size)
{
   std::vector<std::pair<unsigned, std::string>> freq_sorted;
   freq_sorted.reserve(f.size());
   for(auto && i: f)
   {
      freq_sorted.emplace_back(i.second, i.first.to_string(size));
   }
   std::sort(freq_sorted.begin(), freq_sorted.end(),
         [](const auto & l, const auto & r)
   {
      return l.first > r.first;
   });
   unsigned sum = input_size + 1 - size;
   for(auto && i : freq_sorted)
      std::cout << i.second << ' ' << (double(100 * i.first) / sum) << '\n';
   std::cout << '\n';
}

unsigned compute_count(const std::string & input, const std::string& string)
{
   // compute all values, but then only read one value. the problem description
   // requires that we compute all values
   const char * c_str_end = input.c_str() + input.size();
   return calculate(input.c_str(), c_str_end, string.size())[string];
}

void write_single_count(unsigned count, const char * string)
{
   std::cout << count << '\t' << string << '\n';
}

int main()
{
   std::string input;
   char buffer[256];
   while (fgets(buffer, 100, stdin) && memcmp(">THREE", buffer, 6) != 0)
   {
   }
   while (fgets(buffer, 100, stdin) && buffer[0] != '>')
   {
      if (buffer[0] != ';')
      {
         input.append(buffer, strlen(buffer) - 1);
      }
   }

   std::cout << std::setprecision(3) << std::setiosflags(std::ios::fixed);

   // start all tasks at once on background threads
   std::future<unsigned> GGTATTTTAATTTATAGT
      = std::async(launch_policy, compute_count, input, "GGTATTTTAATTTATAGT");
   std::future<unsigned> GGTATTTTAATT
      = std::async(launch_policy, compute_count, input, "GGTATTTTAATT");
   std::future<unsigned> GGTATT
      = std::async(launch_policy, compute_count, input, "GGTATT");
   std::future<unsigned> GGTA
      = std::async(launch_policy, compute_count, input, "GGTA");
   std::future<unsigned> GGT
      = std::async(launch_policy, compute_count, input, "GGT");
   const char * begin = input.data();
   const char * end = input.data() + input.size();
   std::future<HashMap> freq2
      = std::async(launch_policy, calculate, begin, end, 2);
   // do one task on the main thread
   write_frequencies(calculate(begin, end, 1), input.size(), 1);
   // get and write results from background threads
   write_frequencies(freq2.get(), input.size(), 2);
   write_single_count(GGT.get(), "GGT");
   write_single_count(GGTA.get(), "GGTA");
   write_single_count(GGTATT.get(), "GGTATT");
   write_single_count(GGTATTTTAATT.get(), "GGTATTTTAATT");
   write_single_count(GGTATTTTAATTTATAGT.get(), "GGTATTTTAATTTATAGT");
}
