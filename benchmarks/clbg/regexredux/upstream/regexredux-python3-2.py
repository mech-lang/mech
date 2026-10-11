# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Jeremy Zerfas
#
# WARNING: I normally do my programming in other languages. This may not be an
# optimal Python program. Please contribute a better program if you can make a
# better program.

from ctypes import c_char, c_char_p, c_int, c_size_t, c_uint32, \
                   byref, CDLL, create_string_buffer, POINTER
from ctypes.util import find_library
from multiprocessing import Pipe, Process
from multiprocessing.connection import wait
from multiprocessing.sharedctypes import RawArray
from os import cpu_count
from sys import stdin


# We'll be using PCRE2 for our regular expression needs instead of using
# Python's built in regular expression engine because it is significantly
# faster.
PCRE2=CDLL(find_library("pcre2-8"))

# By default, Python assumes that C functions will return int's but that is not
# correct for the following functions which return pointers instead so we tell
# it the correct types here.
PCRE2.pcre2_compile_8.restype=POINTER(c_char)
PCRE2.pcre2_match_data_create_8.restype=POINTER(c_char)
PCRE2.pcre2_get_ovector_pointer_8.restype=POINTER(c_size_t*2)


# Function for searching a src_String for a pattern, replacing it with some
# specified replacement, and storing the result in dst_String which is also
# returned along with the updated dst_String_Length.
def replace(pattern, replacement, src_String, src_String_Length,
  dst_String, dst_String_Length):

	# Compile the pattern and also enable JIT compilation to make matching
	# faster.
	regex=PCRE2.pcre2_compile_8(pattern, c_size_t(len(pattern)), c_uint32(0),
	  byref(c_int()), byref(c_size_t()), None)
	PCRE2.pcre2_jit_compile_8(regex, c_uint32(0x1))

	# Convert dst_String_Length from an int to c_size_t so that the
	# pcre2_substitute_8() function will be able to access it properly.
	dst_String_Length=c_size_t(dst_String_Length)

	# Have PCRE2 do most of the replacement work.
	PCRE2.pcre2_substitute_8(regex, src_String, c_size_t(src_String_Length),
	  c_size_t(0), c_uint32(0x100), None, None,
	  replacement, c_size_t(len(replacement)),
	  dst_String, byref(dst_String_Length))

	PCRE2.pcre2_code_free_8(regex)

	return dst_String, dst_String_Length.value


# This function is used as the main function for the worker subprocesses. The
# worker subprocesses communicate with the manager process via worker_Pipe. The
# worker subprocesses receive tasks from the manager process, complete the
# appropriate task, and then send back a result to the manager process (except
# when the manager process tells the worker subprocess to exit). Tasks are
# performed on the sequences string which is shared between all the processes
# using shared memory.
def process_Task(worker_Pipe, sequences, sequences_Length):

	# When a worker subprocess first starts up, it sends None to the manager
	# process as an indicator that the worker subprocess is ready to process
	# tasks.
	worker_Pipe.send(None)

	# Start an infinte loop to process received tasks accordingly until a
	# request to exit is received.
	while True:

		# Wait for a request.
		task=worker_Pipe.recv()

		# If there is no task to do, then just exit the loop and let this
		# subprocess exit.
		if task is None:
			break

		# If the task contains a tuple and the first element is an int, then
		# this is a task requesting that a match_Count be performed for
		# pattern_To_Count and that this is for the element located at
		# index_For_Pattern_To_Count in count_Info[].
		elif type(task[0]) is int:
			index_For_Pattern_To_Count, pattern_To_Count=task

			match_Data=PCRE2.pcre2_match_data_create_8(c_uint32(1), None)
			match=PCRE2.pcre2_get_ovector_pointer_8(match_Data)

			match.contents[1]=0
			match_Count=0

			# Compile the pattern and also enable JIT compilation to make
			# matching faster.
			regex=PCRE2.pcre2_compile_8(pattern_To_Count,
			  c_size_t(len(pattern_To_Count)), c_uint32(0), byref(c_int()),
			  byref(c_size_t()), None)
			PCRE2.pcre2_jit_compile_8(regex, c_uint32(0x1))

			# Count how many matches there are for pattern in sequences.
			while PCRE2.pcre2_jit_match_8(regex,
			  sequences, c_size_t(sequences_Length),
			  c_size_t(match.contents[1]), c_uint32(0), match_Data, None)>=0:
				match_Count+=1

			PCRE2.pcre2_match_data_free_8(match_Data)
			PCRE2.pcre2_code_free_8(regex)

			# Send the result back to the manager process.
			worker_Pipe.send((index_For_Pattern_To_Count, match_Count))

		# If we got here, then this is a task requesting that replacements be
		# made to the sequences string using all the (pattern, replacement)
		# tuples in the task.
		else:

			# We'll use two strings when doing all the replacements, searching
			# for patterns in prereplace_String and using postreplace_String to
			# store the string after the replacements have been made. After
			# each iteration these two then get swapped. Make both strings 10%
			# larger than the sequences string in order to accomodate some of
			# the replacements causing sequences to grow and also copy the
			# sequences string into prereplace_String for the initial iteration.
			string_Capacities=int(sequences_Length*1.1)
			prereplace_String=create_string_buffer(sequences.raw,
			  string_Capacities)
			prereplace_String_Length=sequences_Length
			postreplace_String=create_string_buffer(string_Capacities)
			postreplace_String_Length=string_Capacities

			# Iterate through all the (pattern, replacement) tuples in the task.
			for pattern, replacement in task:
				postreplace_String, postreplace_String_Length=replace(
				  pattern, replacement,
				  prereplace_String, prereplace_String_Length,
				  postreplace_String, postreplace_String_Length)

				# Swap prereplace_String and postreplace_String in preparation
				# for the next iteration.
				prereplace_String, postreplace_String= \
				  postreplace_String, prereplace_String
				prereplace_String_Length=postreplace_String_Length
				postreplace_String_Length=string_Capacities

			# If any replacements were made, they'll be in prereplace_String
			# instead of postreplace_String because of the swap done after each
			# iteration. Send its length back to the manager process.
			worker_Pipe.send(prereplace_String_Length)


if __name__ == '__main__':
	# Read in input from stdin and also get the input_Length.
	input=stdin.buffer.read()
	input_Length=len(input)


	# Set up some shared memory for the sequences string so that it can be
	# shared between all the processes and make it the same length as the
	# input_Length.
	sequences=RawArray(c_char, input_Length)
	sequences_Length=input_Length

	# Find all sequence descriptions and new lines in input, replace them
	# with empty strings, and store the result in the sequences string.
	sequences, sequences_Length=replace(b">.*\\n|\\n", b"", input, input_Length,
	  sequences, sequences_Length)

	# We'll be using the sequences string instead of the input string from now
	# on so delete our reference to it since this can often free up the memory
	# that was used by it.
	del input


	# Start a worker subprocess on each processor that is available to us and
	# send each worker subprocess the sequences string & a worker_Pipe to use
	# for communicating with the manager process.
	manager_Pipes=[]
	for i in range(cpu_count() or 1):
		manager_Pipe, worker_Pipe=Pipe()
		manager_Pipes.append(manager_Pipe)
		Process(target=process_Task,
		  args=(worker_Pipe, sequences, sequences_Length)).start()


	# Wait for the first worker subproces to send us a None object that
	# indicates it's ready to start processing tasks and then have it start
	# working on performing all the replacements serially.
	manager_Pipes[0].recv()
	manager_Pipes[0].send((
	    (b"tHa[Nt]", b"<4>"),
	    (b"aND|caN|Ha[DS]|WaS", b"<3>"),
	    (b"a[NSt]|BY", b"<2>"),
	    (b"<[^>]*>", b"|"),
	    (b"\\|[^|][^|]*\\|", b"-")
	  ))


	count_Info=[
	    b"agggtaaa|tttaccct",
	    b"[cgt]gggtaaa|tttaccc[acg]",
	    b"a[act]ggtaaa|tttacc[agt]t",
	    b"ag[act]gtaaa|tttac[agt]ct",
	    b"agg[act]taaa|ttta[agt]cct",
	    b"aggg[acg]aaa|ttt[cgt]ccct",
	    b"agggt[cgt]aa|tt[acg]accct",
	    b"agggta[cgt]a|t[acg]taccct",
	    b"agggtaa[cgt]|[acg]ttaccct"
	  ]

	# Now the manger process needs to start managing all the worker subprocesses
	# by waiting for them to become ready to process tasks, handling any results
	# that they send back, sending them any remaining counting tasks, and then
	# finally telling them when it's OK for them to exit (when there are no more
	# tasks to process).
	index_For_Next_Count=0
	while manager_Pipes:

		# Wait for any one of the manager_Pipes to receive something.
		for manager_Pipe in wait(manager_Pipes):
			result=manager_Pipe.recv()

			# If the result is an int, then it's the postreplace_Length that
			# resulted after applying all the replacments that were specified
			# above.
			if type(result) is int:
				postreplace_Length=result

			# If the result is a tuple, then it's the results from one of the
			# counting tasks for the patterns in count_Info[]. The first element
			# is the index of the pattern that the result is for and the second
			# element is the number of matches for it. Add the number of matches
			# to count_Info[].
			elif type(result) is tuple:
				count_Info[result[0]]=[count_Info[result[0]], result[1]]


			# Send the worker subprocess the index_For_Next_Count and pattern to
			# work on if we haven't reached the end of count_Info[] yet.
			if index_For_Next_Count<len(count_Info):
				manager_Pipe.send((index_For_Next_Count,
				  count_Info[index_For_Next_Count]))
				index_For_Next_Count+=1

			# If we have reached the end of count_Info[] then there are no more
			# tasks to start working on so just send the worker subprocess None
			# to indicate it can exit and also stop keeping track of the
			# manger_Pipe for it.
			else:
				manager_Pipe.send(None)
				manager_Pipes.remove(manager_Pipe)


	# Print the match_Count for each pattern in count_Info[].
	for pattern, match_Count in count_Info:
		print(pattern.decode(), match_Count)

	# Print the size of the original input, the size of the input without the
	# sequence descriptions & new lines, and the size after having made all the
	# replacements.
	print()
	print(input_Length)
	print(sequences_Length)
	print(postreplace_Length)
