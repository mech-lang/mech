// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
// Contributed by Paul Kitchin

#include <iostream>
#include <iterator>
#include <vector>

std::size_t line_length = 60;

char complement(char character)
{
   static char const complement_map[] =
   {
      '\0', 'T', 'V', 'G', 'H', '\0', '\0', 'C', 'D', '\0', '\0', 'M', '\0', 'K', 'N', '\0', '\0', '\0', 'Y', 'S', 'A', 'A', 'B', 'W', '\0', 'R'
   };
   return complement_map[character & 0x1f];
}

struct chunk
{
   chunk()
      :
      previous(0),
      next(0),
      length(0)
   {
   }
   chunk(chunk * previous)
      :
      previous(previous),
      next(0),
      length(0)
   {
      previous->next = this;
   }
   chunk * previous;
   chunk * next;
   unsigned short length;
   char data[65526];
};

void write_reverse_complement(chunk * begin, chunk * end)
{
   chunk * start = begin;
   char * begin_char = begin->data;
   char * end_char = end->data + end->length - 1;
   while (begin != end || begin_char < end_char)
   {
      char temp = complement(*begin_char);
      *begin_char++ = complement(*end_char);
      *end_char-- = temp;
      if (*begin_char == '\n')
      {
         ++begin_char;
      }
      if (*end_char == '\n')
      {
         --end_char;
      }
      if (begin_char == begin->data + begin->length)
      {
         begin = begin->next;
         begin_char = begin->data;
         if (*begin_char == '\n')
         {
            ++begin_char;
         }
      }
      if (end_char == end->data - 1)
      {
         end = end->previous;
         end_char = end->data + end->length - 1;
         if (*end_char == '\n')
         {
            --end_char;
         }
      }
   }
   while (start)
   {
      std::cout.write(start->data, start->length);
      chunk * last = start;
      start = start->next;
      delete last;
   }
   std::cout.put('\n');
}

int main()
{
   std::cin.sync_with_stdio(false);
   std::cout.sync_with_stdio(false);
   while (!std::cin.eof())
   {
      std::string header;
      std::getline(std::cin, header);
      std::cout << header << '\n';
      chunk * start = new chunk();
      chunk * end = start;
      while (!std::cin.eof() && std::cin.peek() != '>')
      {
         for (int line = 0; line < 1074 && !std::cin.eof() && std::cin.peek() != '>'; ++line)
         {
            std::cin.getline(end->data + end->length, line_length + 1);
            end->length += std::cin.gcount();
            *(end->data + end->length - 1) = '\n';
         }
         if (!std::cin.eof() && std::cin.peek() != '>')
         {
            end = new chunk(end);
         }
      }
      --end->length;
      write_reverse_complement(start, end);
   }
}
