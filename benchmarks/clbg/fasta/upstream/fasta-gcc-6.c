// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Contributed by Jeremy Zerfas

// This controls the width of lines that are output by this program.
#define MAXIMUM_LINE_WIDTH	60

// This program will generate the random nucleotide sequences in parallel which
// are worked on in blocks of lines. The number of lines in those blocks is
// controlled by this setting.
#define LINES_PER_BLOCK 1024

#define CHARACTERS_PER_BLOCK (MAXIMUM_LINE_WIDTH*LINES_PER_BLOCK)

// The numbers of blocks to use is controlled by this setting. This program
// limits itself to using at most three threads (see explanation further below)
// so similarly only three blocks are used.
#define BLOCKS_TO_USE 3

#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

#ifdef _OPENMP
	#include <omp.h>
#endif

// intptr_t should be the native integer type on most sane systems.
typedef intptr_t intnative_t;

typedef struct{
	char letter;
	float probability;
} nucleotide_info;


// Repeatedly print string_To_Repeat until it has printed
// number_Of_Characters_To_Create. The output is also wrapped to
// MAXIMUM_LINE_WIDTH columns.
static void repeat_And_Wrap_String(const char string_To_Repeat[],
  const intnative_t number_Of_Characters_To_Create){
	const intnative_t string_To_Repeat_Length=strlen(string_To_Repeat);

	// Create an extended_String_To_Repeat which is a copy of string_To_Repeat
	// but extended with another copy of the first MAXIMUM_LINE_WIDTH characters
	// of string_To_Repeat appended to the end. Later on this allows us to
	// generate a line of output just by doing simple memory copies using an
	// appropriate offset into extended_String_To_Repeat.
	char extended_String_To_Repeat[string_To_Repeat_Length+MAXIMUM_LINE_WIDTH];
	for(intnative_t column=0; column<string_To_Repeat_Length+MAXIMUM_LINE_WIDTH;
	  column++)
		extended_String_To_Repeat[column]=
		  string_To_Repeat[column%string_To_Repeat_Length];
	intnative_t offset=0;

	char line[MAXIMUM_LINE_WIDTH+1];
	line[MAXIMUM_LINE_WIDTH]='\n';

	for(intnative_t current_Number_Of_Characters_To_Create=
	  number_Of_Characters_To_Create;
	  current_Number_Of_Characters_To_Create>0;){
		// Figure out the length of the line we need to write. If it's less than
		// MAXIMUM_LINE_WIDTH then we also need to add a line feed in the right
		// spot too.
		intnative_t line_Length=MAXIMUM_LINE_WIDTH;
		if(current_Number_Of_Characters_To_Create<MAXIMUM_LINE_WIDTH){
			line_Length=current_Number_Of_Characters_To_Create;
			line[line_Length]='\n';
		}

		memcpy(line, extended_String_To_Repeat+offset, line_Length);

		// Update the offset, reducing it by string_To_Repeat_Length if
		// necessary.
		offset+=line_Length;
		if(offset>string_To_Repeat_Length)
			offset-=string_To_Repeat_Length;

		// Output the line to stdout and update the
		// current_Number_Of_Characters_To_Create.
		fwrite(line, line_Length+1, 1, stdout);
		current_Number_Of_Characters_To_Create-=line_Length;
	}
}


// Generate a floating point pseudorandom number from 0.0 to max using a linear
// congruential generator.
#define IM		139968
#define IA		3877
#define IC		29573
uint32_t seed=42;
inline float get_LCG_Pseudorandom_Number(const float max){
	seed=(seed*IA + IC)%IM;
	return max/IM*seed;
}


// Output any blocks that are ready to be output. Checks the block_Statuses of
// up to number_Of_Blocks starting at next_Block_To_Output.
void output_Blocks(char (* const blocks)[CHARACTERS_PER_BLOCK+LINES_PER_BLOCK],
  intnative_t * const block_Statuses, intnative_t * const next_Block_To_Output,
  const intnative_t number_Of_Blocks){

	// Iterate over all the blocks at the start of the circular queue which are
	// ready to be output.
	for(; block_Statuses[*next_Block_To_Output]>0;
	  *next_Block_To_Output=(*next_Block_To_Output+1)%number_Of_Blocks){

		// Iterate over each line in the next_Block_To_Output.
		char * line=blocks[*next_Block_To_Output];
		for(intnative_t block_Characters_Left_To_Output=
		  block_Statuses[*next_Block_To_Output];
		  block_Characters_Left_To_Output>0;){
			// Add a line feed to the end of the line.
			line[MAXIMUM_LINE_WIDTH]='\n';

			// Determine what the line_Length should be and if necessary add a
			// line feed at the end.
			intnative_t line_Length=MAXIMUM_LINE_WIDTH;
			if(block_Characters_Left_To_Output<MAXIMUM_LINE_WIDTH){
				line_Length=block_Characters_Left_To_Output;
				line[line_Length]='\n';
			}

			// Write one line of the block.
			fwrite(line, line_Length+1, 1, stdout);

			// Decrement block_Characters_Left_To_Output and advance to the next
			// line.
			block_Characters_Left_To_Output-=line_Length;
			line+=line_Length+1;
		}

		// Mark the block as unused now.
		block_Statuses[*next_Block_To_Output]=-1;
	}
}


// Print a pseudorandom DNA sequence that is number_Of_Characters_To_Create
// characters long and made up of the nucleotides specified in
// nucleotides_Information and occurring at the frequencies specified in
// nucleotides_Information. The output is also wrapped to MAXIMUM_LINE_WIDTH
// columns.
static void generate_And_Wrap_Pseudorandom_DNA_Sequence(
  const nucleotide_info nucleotides_Information[],
  const intnative_t number_Of_Nucleotides,
  const intnative_t number_Of_Characters_To_Create){

	// Cumulate the probabilities. Note that the probability is being multiplied
	// by IM because later on we'll also be calling the random number generator
	// with a value that is multiplied by IM. Since the random number generator
	// does a division by IM this allows the compiler to cancel out the
	// multiplication and division by IM with each other without requiring any
	// changes to the random number generator code whose code was explicitly
	// defined in the rules.
	float cumulative_Probabilities[number_Of_Nucleotides],
	  cumulative_Probability=0.0;
	for(intnative_t i=0; i<number_Of_Nucleotides; i++){
		cumulative_Probability+=nucleotides_Information[i].probability;
		cumulative_Probabilities[i]=cumulative_Probability*IM;
	}

	// blocks is a circular queue that stores the blocks while they are being
	// processed. next_Block_To_Output contains the index of the first block in
	// the queue which is also the next block that should be output once it is
	// ready.
	char blocks[BLOCKS_TO_USE][CHARACTERS_PER_BLOCK+LINES_PER_BLOCK];
	intnative_t next_Block_To_Output=0;

	// block_Statuses contains a status value for each block in the circular
	// queue.
	//  -A value of -1 means that block is not in use.
	//  -A value of 0 means that block is in use and being processed.
	//  -A positive value means that block is ready to be output and the value
	//   is its length.
	intnative_t block_Statuses[BLOCKS_TO_USE];
	for(intnative_t i=0; i<BLOCKS_TO_USE; block_Statuses[i++]=-1);

	intnative_t current_Number_Of_Characters_To_Create=
	  number_Of_Characters_To_Create;

	// Limit the number_Of_Threads_To_Use to three threads since the bottleneck
	// for this program is the speed at which the pseudorandom generator can be
	// ran at which is only fast enough to keep about two other threads busy.
	// Using more threads will start slowing down the program due to the
	// overhead from additional thread management and resource usage. Using more
	// threads will also use more CPU time too since normally waiting OpenMP
	// threads will use spinlocks.
	#ifdef _OPENMP
		intnative_t number_Of_Threads_To_Use=omp_get_num_procs();
		if(number_Of_Threads_To_Use>3) number_Of_Threads_To_Use=3;
		omp_set_num_threads(number_Of_Threads_To_Use);
	#endif

	#pragma omp parallel for schedule(guided)
	for(intnative_t current_Block_Number=0; current_Block_Number<
	  (number_Of_Characters_To_Create+CHARACTERS_PER_BLOCK-1)/
	  CHARACTERS_PER_BLOCK; current_Block_Number++){

		intnative_t block_To_Use, block_Length;
		float pseudorandom_Numbers[CHARACTERS_PER_BLOCK];

		// Only one thread can be outputting blocks or generating pseudorandom
		// numbers at a time in order to ensure they are done in the correct
		// order.
		#pragma omp critical
		{
			// Find the first unused block (if any) and set that as the
			// block_To_Use for outputting the nucleotide sequence to.
			block_To_Use=next_Block_To_Output;
			for(intnative_t i=0; i<BLOCKS_TO_USE; i++,
			  block_To_Use=(block_To_Use+1)%BLOCKS_TO_USE){
				if(block_Statuses[block_To_Use]==-1)
					break;
			}

			// If no unused block was found then block_To_Use will be restored
			// to next_Block_To_Output and we will have to wait for it to finish
			// processing, output that block, and then use that block.
			while(block_Statuses[block_To_Use]==0){
				#pragma omp flush(block_Statuses)
			}

			// Output any blocks that are ready to be output.
			output_Blocks(blocks, block_Statuses, &next_Block_To_Output,
			  BLOCKS_TO_USE);

			// Update the status for block_To_Use to reflect that it is now
			// being processed.
			block_Statuses[block_To_Use]++;

			// Figure out what the block_Length should be and decrement
			// current_Number_Of_Characters_To_Create by that amount.
			block_Length=CHARACTERS_PER_BLOCK;
			if(current_Number_Of_Characters_To_Create<CHARACTERS_PER_BLOCK)
				block_Length=current_Number_Of_Characters_To_Create;
			current_Number_Of_Characters_To_Create-=block_Length;

			// Get the pseudorandom_Numbers to use for this block.
			for(intnative_t pseudorandom_Number_Index=0;
			  pseudorandom_Number_Index<block_Length;
			  pseudorandom_Numbers[pseudorandom_Number_Index++]=
			  get_LCG_Pseudorandom_Number(IM));
		}


		// Start processing the pseudorandom_Numbers and generate the
		// corresponding block of nucleotides that will be output later by
		// filling block_To_Use with characters from nucleotides_Information[]
		// that are selected by looking up the pseudorandom number.
		char * line=blocks[block_To_Use];
		for(intnative_t column=0, pseudorandom_Number_Index=0;
		  pseudorandom_Number_Index<block_Length; pseudorandom_Number_Index++){
			const float r=pseudorandom_Numbers[pseudorandom_Number_Index];

			// Count the number of nucleotides with a probability less than what
			// was selected by the random number generator and then use that
			// count as an index for the nucleotide to select. It's arguable
			// whether this qualifies as a linear search but I guess you can say
			// that you're doing a linear search for all the nucleotides with a
			// probability less than what was selected by the random number
			// generator and then just counting how many matches were found.
			// With a small number of nucleotides this can be faster than doing
			// a more normal linear search (although in some cases it may
			// generate different results) and a couple of the other programs
			// already do this as well so we will too.
			intnative_t count=0;
			for(intnative_t i=0; i<number_Of_Nucleotides; i++)
				if(cumulative_Probabilities[i]<=r)
					count++;

			line[column]=nucleotides_Information[count].letter;

			// If we reach the end of the line, reset the column counter and
			// advance to the next line.
			if(++column==MAXIMUM_LINE_WIDTH){
				column=0;
				line+=MAXIMUM_LINE_WIDTH+1;
			}
		}


		// Update the block_Statuses so that this block_To_Use gets output
		// later.
		block_Statuses[block_To_Use]=block_Length;
	}

	// Output the remaining blocks.
	output_Blocks(blocks, block_Statuses, &next_Block_To_Output, BLOCKS_TO_USE);
}


int main(int argc, char ** argv){
	const intnative_t n=atoi(argv[1]);

	fputs(">ONE Homo sapiens alu\n", stdout);
	const char homo_Sapiens_Alu[]=
	  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTC"
	  "AGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCG"
	  "TGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGG"
	  "AGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
	repeat_And_Wrap_String(homo_Sapiens_Alu, 2*n);

	fputs(">TWO IUB ambiguity codes\n", stdout);
	nucleotide_info iub_Nucleotides_Information[]={
	  {'a', 0.27}, {'c', 0.12}, {'g', 0.12}, {'t', 0.27}, {'B', 0.02},
	  {'D', 0.02}, {'H', 0.02}, {'K', 0.02}, {'M', 0.02}, {'N', 0.02},
	  {'R', 0.02}, {'S', 0.02}, {'V', 0.02}, {'W', 0.02}, {'Y', 0.02}};
	generate_And_Wrap_Pseudorandom_DNA_Sequence(iub_Nucleotides_Information,
	  sizeof(iub_Nucleotides_Information)/sizeof(nucleotide_info), 3*n);

	fputs(">THREE Homo sapiens frequency\n", stdout);
	nucleotide_info homo_Sapien_Nucleotides_Information[]={
	  {'a', 0.3029549426680}, {'c', 0.1979883004921},
	  {'g', 0.1975473066391}, {'t', 0.3015094502008}};
	generate_And_Wrap_Pseudorandom_DNA_Sequence(
	  homo_Sapien_Nucleotides_Information,
	  sizeof(homo_Sapien_Nucleotides_Information)/sizeof(nucleotide_info), 5*n);

	return 0;
}
