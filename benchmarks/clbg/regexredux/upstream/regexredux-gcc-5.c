// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Jeremy Zerfas 
// modified by Zoltan Herczeg

#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#define PCRE2_CODE_UNIT_WIDTH 8
#include "pcre2.h"

typedef struct {
    PCRE2_UCHAR *data;
    PCRE2_SIZE capacity, size;
} string;


// Function for searching a src_String for a pattern, replacing it with some
// specified replacement, and storing the result in dst_String.
static void replace(char const * const pattern, char const * const replacement
  , string const * const src_String, string * const dst_String
  , pcre2_match_context * const mcontext, pcre2_match_data * mdata){

    int errorcode;
    PCRE2_SIZE erroroffset, pos=0;
    int const replacement_Size=strlen(replacement);
    PCRE2_SIZE *match=pcre2_get_ovector_pointer(mdata);

    // Compile and study pattern.
    pcre2_code * regex=pcre2_compile((PCRE2_SPTR)pattern, PCRE2_ZERO_TERMINATED
      , 0, &errorcode, &erroroffset, NULL);
    pcre2_jit_compile(regex, PCRE2_JIT_COMPLETE);

    // Find each match of the pattern in src_String and append the characters
    // preceding each match and the replacement text to dst_String.
    while(pcre2_jit_match(regex, src_String->data, src_String->size, pos, 0
      , mdata, mcontext)>=0){

        // Allocate more memory for dst_String if there is not enough space for
        // the characters preceding the match and the replacement text.
        while(dst_String->size+match[0]-pos+replacement_Size
          >dst_String->capacity)
            dst_String->data=realloc(dst_String->data, dst_String->capacity*=2);

        // Append the characters preceding the match and the replacement text to
        // dst_String and update the size of dst_String.
        memcpy(dst_String->data+dst_String->size, src_String->data+pos
          , match[0]-pos);
        memcpy(dst_String->data+dst_String->size+match[0]-pos, replacement
          , replacement_Size);
        dst_String->size+=match[0]-pos+replacement_Size;

        // Update pos to continue searching after the current match.
        pos=match[1];
    }

    pcre2_code_free(regex);

    // Allocate more memory for dst_String if there is not enough space for
    // the characters following the last match (or the entire src_String if
    // there was no match).
    while(dst_String->size+src_String->size-pos>dst_String->capacity)
        dst_String->data=realloc(dst_String->data, dst_String->capacity*=2);

    // Append the characters following the last match (or the entire src_String
    // if there was no match) to dst_String and update the size of dst_String.
    memcpy(dst_String->data+dst_String->size, src_String->data+pos
      , src_String->size-pos);
    dst_String->size+=src_String->size-pos;
}


int main(void){
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

    string input={malloc(16384), 16384}, sequences={malloc(16384), 16384};
    int postreplace_Size;


    // Read in input from stdin until we reach the end or encounter an error.
    for(int bytes_Read;
      (bytes_Read=fread(input.data+input.size, 1, input.capacity-input.size
      , stdin))>0;)
        // Update the size of input to reflect the newly read input and if
        // we've reached the full capacity of the input string then also double
        // its size.
        if((input.size+=bytes_Read)==input.capacity)
            input.data=realloc(input.data, input.capacity*=2);


    #pragma omp parallel
    {
        pcre2_match_context * mcontext=pcre2_match_context_create(NULL);
        pcre2_jit_stack * const stack=pcre2_jit_stack_create(16384, 16384, NULL);
        pcre2_jit_stack_assign(mcontext, NULL, stack);

        pcre2_match_data * mdata=pcre2_match_data_create(16, NULL);

        // Find all sequence descriptions and new lines in input, replace them
        // with empty strings, and store the result in the sequences string.
        #pragma omp single
        {
            replace(">.*\\n|\\n", "", &input, &sequences, mcontext, mdata);

            free(input.data);
        }


        // Have one thread start working on performing all the replacements
        // serially.
        #pragma omp single nowait
        {
            // We'll use two strings when doing all the replacements, searching
            // for patterns in prereplace_String and using postreplace_String to
            // store the string after the replacements have been made. After
            // each iteration these two then get swapped. Start out with both
            // strings having the same capacity as the sequences string and also
            // copy the sequences string into prereplace_String for the initial
            // iteration.
            string prereplace_String={
                malloc(sequences.capacity), sequences.capacity, sequences.size
              }, postreplace_String={
                malloc(sequences.capacity), sequences.capacity
              };
            memcpy(prereplace_String.data, sequences.data, sequences.size);

            // Iterate through all the replacement patterns and their
            // replacements in replace_Info[].
            for(int i=0; i<sizeof(replace_Info)/sizeof(char * [2]); i++){

                replace(replace_Info[i][0], replace_Info[i][1]
                  , &prereplace_String, &postreplace_String, mcontext, mdata);

                // Swap prereplace_String and postreplace_String in preparation
                // for the next iteration.
                string const temp=prereplace_String;
                prereplace_String=postreplace_String;
                postreplace_String=temp;

                postreplace_String.size=0;
            }

            // If any replacements were made, they'll be in prereplace_String
            // instead of postreplace_String because of the swap done after each
            // iteration. Copy its size to postreplace_Size.
            postreplace_Size=prereplace_String.size;

            free(prereplace_String.data);
            free(postreplace_String.data);
        }


        // Iterate through all the count patterns in count_Info[] and perform
        // the counting for each one on a different thread if available.
        #pragma omp for schedule(dynamic) ordered
        for(int i=0; i<sizeof(count_Info)/sizeof(char *); i++){

            int errorcode, count=0;
            PCRE2_SIZE erroroffset, pos=0;
            PCRE2_SIZE *match=pcre2_get_ovector_pointer(mdata);

            // Compile and study pattern.
            pcre2_code * regex=pcre2_compile((PCRE2_SPTR)count_Info[i]
              , PCRE2_ZERO_TERMINATED, 0, &errorcode, &erroroffset, NULL);
            pcre2_jit_compile(regex, PCRE2_JIT_COMPLETE);

            // Find each match of the pattern in the sequences string and
            // increment count for each match.
            while(pcre2_jit_match(regex, sequences.data, sequences.size
              , pos, 0, mdata, mcontext)>=0){

                count++;

                // Update pos to continue searching after the current match.
                pos=match[1];
            }

            pcre2_code_free(regex);

            // Print the count for each pattern in the correct order.
            #pragma omp ordered
            printf("%s %d\n", count_Info[i], count);
        }


        pcre2_match_context_free(mcontext);
        pcre2_jit_stack_free(stack);
        pcre2_match_data_free(mdata);
    }


    free(sequences.data);

    // Print the size of the original input, the size of the input without the
    // sequence descriptions & new lines, and the size after having made all the
    // replacements.
    printf("\n%d\n%d\n%d\n", (int)input.size, (int)sequences.size
      , postreplace_Size);
    return 0;
}
