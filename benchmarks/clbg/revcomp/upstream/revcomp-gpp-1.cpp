/* ------------------------------------------------------------------ */
/* The Computer Language Benchmarks Game                              */
/* https://salsa.debian.org/benchmarksgame-team/benchmarksgame/                                 */
/*                                                                    */
/* Contributed by Anthony Borla                                       */
/* Modified by Vaclav Haisman                                         */
/* Changed to match style of Perl example: Greg Buchholz              */
/* ------------------------------------------------------------------ */

#include <cctype>
#include <string>
#include <algorithm>
#include <iterator>
#include <iostream>
using namespace std;

const int LINELENGTH = 60;

typedef string Header;
typedef string Segment;

inline char complement(char element)
{
  static const char charMap[] =
    {
      'T', 'V', 'G', 'H', '\0', '\0', 'C', 'D', '\0', '\0', 'M', '\0', 'K',
      'N', '\0', '\0', '\0', 'Y', 'S', 'A', 'A', 'B', 'W', '\0', 'R', '\0'
    };

  return charMap[toupper(element) - 'A'];
}

void print_revcomp(Header const& header, Segment const& seg, ostream& out = std::cout)
{
    out << header << "\n";
    
    Segment comp(seg.rbegin(),seg.rend());
    transform(comp.begin(),comp.end(), comp.begin(), complement);
   
    size_t i = 0;
    size_t stop = comp.length()/LINELENGTH + ((comp.length()%LINELENGTH)?1:0);
    
    while(i < stop)
        out << comp.substr(i++*LINELENGTH,LINELENGTH) << "\n";
}

int main ()
{
  ios_base::sync_with_stdio(false);

  Segment line, segment; 
  Header header;

  while (getline(cin, line))
  {
      if (line[0] == '>')
      {
          if (! segment.empty())
            print_revcomp(header, segment);
          header = line;
          segment.clear();
      }
      else
          segment += line;
  }
  print_revcomp(header, segment);

  return 0;
}
