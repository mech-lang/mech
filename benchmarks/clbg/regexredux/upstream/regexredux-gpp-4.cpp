/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Filip Sajdak
*/

#include <pcre.h>
#include <iostream>
#include <memory>
#include <string>
#include <string_view>

template <typename T>
auto reserve(size_t size) {
   T out;
   out.reserve(size);
   return out;
}

template <auto initial_size = 16384, auto buffer_size = initial_size>
auto load(std::istream& in) {
      auto str = reserve<std::string>(initial_size);
      auto buffer = std::array<char, buffer_size>();

      while (in) {
         in.read(buffer.data(), buffer.size());
         str.append(buffer.cbegin(), buffer.cbegin()+in.gcount());
      }
      
      return str;
}

template<typename T, typename Deleter>
auto make_unique_with_deleter(T* ptr, Deleter&& deleter)
{
    return std::unique_ptr<T, Deleter>(ptr, std::forward<Deleter>(deleter));
} 

   auto make_regex(const std::string_view pattern) {
      char const* error;
      int offset;
      return make_unique_with_deleter(
         pcre_compile(pattern.data(), 0, &error, &offset, NULL), pcre_free);
   }
   
   auto make_aid(const pcre* regex) {
      char const* error;
      return make_unique_with_deleter(
         pcre_study(regex, PCRE_STUDY_JIT_COMPILE, &error), pcre_free_study);
   }

static void replace(const std::string_view pattern, const std::string_view replacement,
       const std::string_view source, std::string& output, pcre_jit_stack* const stack){

   const auto regex = make_regex(pattern);
   const auto aid   = make_aid(regex.get());

   int pos = 0;
   
   for(int match[3]; pcre_jit_exec(regex.get(), aid.get(), source.data(),
         source.size(), pos, 0, match, 3, stack) >= 0; pos = match[1]){
      output.append(std::cbegin(source) + pos, std::cbegin(source) + match[0]);
      output.append(std::cbegin(replacement), std::cend(replacement));
    }

   output.append(std::cbegin(source) + pos, std::cend(source));
}


int main(void){
   std::ios::sync_with_stdio(false);
   
    char const * const count_Info[]={
        "agggtaaa|tttaccct",
        "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct",
        "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct",
        "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct"
      }, * const replace_Info[][2]={
        {"tHa[Nt]", "<4>"},
        {"aND|caN|Ha[DS]|WaS", "<3>"},
        {"a[NSt]|BY", "<2>"},
        {"<[^>]*>", "|"},
        {"\\|[^|][^|]*\\|", "-"}
      };

   auto sequences = reserve<std::string>(16384);
    size_t postreplace_Size = 0;

   std::string input = load(std::cin);

    #pragma omp parallel
    {
      auto stack = make_unique_with_deleter(
         pcre_jit_stack_alloc(16384, 16384), pcre_jit_stack_free);

        #pragma omp single
        {
            replace(">.*\\n|\\n", "", input, sequences, stack.get());
        }

        #pragma omp single nowait
        {
            auto prereplace_String = sequences;
            auto postreplace_String = reserve<std::string>(sequences.capacity());
         
         for ( const auto& [pattern, replacement] : replace_Info ) {
            postreplace_String.clear();
            replace(pattern, replacement, 
               prereplace_String, postreplace_String, stack.get());
            std::swap(prereplace_String, postreplace_String);
            }

            postreplace_Size = prereplace_String.size();
        }

        #pragma omp for schedule(dynamic) ordered
        for(auto i=0u; i < std::size(count_Info); i++){

         auto regex = make_regex(count_Info[i]);
         auto aid = make_aid(regex.get());
         
         int count = 0;
         for(int pos = 0, match[3]; pcre_jit_exec(regex.get(), aid.get(),
               sequences.data(), sequences.size(), pos, 0, match, 3,
               stack.get()) >= 0; pos=match[1]) {
            ++count;
            }

            #pragma omp ordered
            printf("%s %d\n", count_Info[i], count);
        }
    }

    printf("\n%zu\n%zu\n%zu\n", input.size(), sequences.size(), postreplace_Size);
    return 0;
}
