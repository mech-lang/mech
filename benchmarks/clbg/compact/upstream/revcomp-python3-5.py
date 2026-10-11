# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Jeremy Zerfas.

from multiprocessing import Queue, Semaphore, Condition, Value, Process
from sys import stdin, stdout
# See comment farther down below regarding why it might be a good idea to call
# set_inheritable() with a file descriptor for stdin.
from os import cpu_count, read, write #, set_inheritable


def process_Sequences(info_For_Remaining_Sequences_Data_To_Process
  , stdin_File_Descriptor, CPU_Cores_Available_For_Processing_Sequences
  , sequence_Was_Written_Condition, next_Sequence_Number_To_Output):

	# This string/character array is used to convert characters into the
	# complementing character. Note that some of the reverse complementing code
	# also requires newlines to remain unchanged when complemented which is why
	# the eleventh character is set to a newline.
	COMPLEMENT_LOOKUP=(
	  b"          \n                                                     "
	  #  ABCDEFGHIJKLMNOPQRSTUVWXYZ      abcdefghijklmnopqrstuvwxyz
	  b" TVGH  CD  M KN   YSAABW R       TVGH  CD  M KN   YSAABW R      "
	  b"                                                                "
	  b"                                                                ")

	# This controls the size of reads from the input.
	READ_SIZE=65536

	# This defines how many characters (not including the newline) a full line
	# of input should have.
	LINE_LENGTH=60


	# Keep processing sequences until we exit this loop when we get None for
	# remaining_Sequences_Data_To_Process.	
	while True:

		# Get the remaining_Sequences_Data_To_Process and the sequence_Number
		# for the first sequence in it (assuming there still is any
		# remaining_Sequences_Data_To_Process).
		remaining_Sequences_Data_To_Process, sequence_Number= \
		  info_For_Remaining_Sequences_Data_To_Process.get()


		# Break out of the loop if there is no
		# remaining_Sequences_Data_To_Process. Also put a None, None tuple back
		# into info_For_Remaining_Sequences_Data_To_Process so that other
		# processes can also determine that there is no more
		# remaining_Sequences_Data_To_Process and release the CPU core this
		# process was using for processing sequences.
		if remaining_Sequences_Data_To_Process is None:
			info_For_Remaining_Sequences_Data_To_Process.put((None, None))
			CPU_Cores_Available_For_Processing_Sequences.release()
			break


		# The next loop will be adding data for the current sequence and will
		# stop adding data for the current sequence once it encounters a ">"
		# that indicates the start of a new sequence or it reaches the end of
		# input. If remaining_Sequences_Data_To_Process isn't an empty string,
		# then the first character will either be a non-">" character for data
		# that showed up before the first sequence header or a ">" for the start
		# of the current sequence. We need to remove the first character if it's
		# a ">" so that the next loop can function properly and if it's a
		# non-">" character we'll just be ignoring it later anyway (any data
		# before the initial sequence header is considered part of sequence zero
		# which the later code knows to just ignore). So regardless of what
		# character it is we just end up removing it from
		# remaining_Sequences_Data_To_Process and adding it to sequence.
		sequence=bytearray()
		sequence+=remaining_Sequences_Data_To_Process[0:1]
		remaining_Sequences_Data_To_Process= \
		  remaining_Sequences_Data_To_Process[1:]

		# Keep adding data from remaining_Sequences_Data_To_Process and then
		# from stdin to sequence until we encounter a ">" indicating the start
		# of a new sequence or we encounter the end of input.
		while True:

			# If we encounter a ">" then add everything before it to the current
			# sequence and add the ">" and everything following it to
			# remaining_Sequences_Data_To_Process which will then be added to
			# info_For_Remaining_Sequences_Data_To_Process later.
			if b">" in remaining_Sequences_Data_To_Process:
				preceding_Bytes, _, following_Bytes= \
				  remaining_Sequences_Data_To_Process.partition(b">")
				sequence+=preceding_Bytes
				remaining_Sequences_Data_To_Process=b">"+following_Bytes
				break

			sequence+=remaining_Sequences_Data_To_Process

			remaining_Sequences_Data_To_Process=read(stdin_File_Descriptor
			  , READ_SIZE)

			# Exit the loop if there is no more
			# remaining_Sequences_Data_To_Process. The following code will then
			# process any non-zero sequences and also add a None, None tuple to
			# info_For_Remaining_Sequences_Data_To_Process so that processes can
			# determine that the end of input has been reached.
			if not remaining_Sequences_Data_To_Process:
				break


		# If there is still any remaining_Sequences_Data_To_Process, add it to
		# info_For_Remaining_Sequences_Data_To_Process for one of the processes
		# to handle later. Otherwise add a None, None tuple to
		# info_For_Remaining_Sequences_Data_To_Process so that processes can
		# determine that the end of input has been reached.
		if remaining_Sequences_Data_To_Process:
			info_For_Remaining_Sequences_Data_To_Process.put( \
			  (remaining_Sequences_Data_To_Process, sequence_Number+1))

			# If the current sequence is anything but sequence zero, then also
			# try acquiring a CPU core for processing sequences and start
			# another process to process_Sequences while we start reverse
			# complementing and outputting the current sequence in just a moment.
			if sequence_Number>0 \
			  and CPU_Cores_Available_For_Processing_Sequences.acquire(False):
				Process(target=process_Sequences, args=(
				  info_For_Remaining_Sequences_Data_To_Process
				  , stdin_File_Descriptor
				  , CPU_Cores_Available_For_Processing_Sequences
				  , sequence_Was_Written_Condition
				  , next_Sequence_Number_To_Output)).start()
		else:
			info_For_Remaining_Sequences_Data_To_Process.put((None, None))


		# If the current sequence is anything but sequence zero, then now we
		# start reverse complementing and outputting that sequence.
		if sequence_Number>0:

			# Get the sequence_Header and save everything after the new line in
			# the header to temporary_Sequence_Data.
			sequence_Header, _, temporary_Sequence_Data= \
			  sequence.partition(b"\n")
			del sequence


			# Check to see if all the lines in the sequence data are an optimal
			# length (LINE_LENGTH characters plus a newline). If all the lines
			# are an optimal length, then this makes it much easier to reverse
			# the sequence since we won't need to deal with having to move
			# newlines around to their correct positions during/after the
			# reversal step. This is even more important for something like
			# CPython since this will allow us to use a library function (that
			# is typically implemented in a faster programming language) to do
			# the entire reversal step instead of having to use slower Python
			# code to do at least part of the the reversal step.
			if len(temporary_Sequence_Data)%(LINE_LENGTH+1)==0:

				modified_Sequence_Data= \
				  temporary_Sequence_Data.translate(COMPLEMENT_LOOKUP)
				modified_Sequence_Data.reverse()
				# The newline that was originally at the end of the
				# temporary_Sequence_Data will now be at the start of the
				# modified_Sequence_Data which is good since the newline that
				# originally was at the start of the sequence data got
				# removed while partitioning the sequence above. Now just append
				# another newline to the end of the modified_Sequence_Data since
				# the newline that was originally at the start of the sequence
				# data got removed and hence never was moved to the end of the
				# modified_Sequence_Data when it was reversed.
				modified_Sequence_Data+=b"\n"

			else:

				# The lines in the sequence data are not an optimal length so in
				# this case we can't just have library functions do all of the
				# work and we'll instead need to use a little more code to help
				# out with moving newlines into their correct postions. We start
				# out by removing all newlines (that are in incorrect positions)
				# and then after reversing the stripped sequence data we will
				# then create the correct modified_Sequence_Data by continuously
				# appending data from the stripped sequence data with newlines
				# at the appropriate positions.
				temporary_Sequence_Data= \
				  temporary_Sequence_Data.translate(COMPLEMENT_LOOKUP, b"\n")
				temporary_Sequence_Data.reverse()
				# In addition to adding newlines after each line of the
				# modified_Sequence_Data, we also need to put one more at the
				# start.
				modified_Sequence_Data=bytearray(b"\n")
				for i in range(0, len(temporary_Sequence_Data), LINE_LENGTH):
					modified_Sequence_Data+= \
					  temporary_Sequence_Data[i:i+LINE_LENGTH]
					modified_Sequence_Data+=b"\n"

			del temporary_Sequence_Data


			# Wait for our turn to output the sequence_Header and
			# modified_Sequence_Data and then update
			# next_Sequence_Number_To_Output and notify any other processes that
			# may be waiting to output following sequences.
			with sequence_Was_Written_Condition:
				while next_Sequence_Number_To_Output.value<sequence_Number:
					sequence_Was_Written_Condition.wait()

				write(stdout.fileno(), sequence_Header)
				write(stdout.fileno(), modified_Sequence_Data)

				next_Sequence_Number_To_Output.value+=1
				sequence_Was_Written_Condition.notify_all()

			del modified_Sequence_Data


if __name__=="__main__":

	# We need somewhere to keep a tuple with
	# info_For_Remaining_Sequences_Data_To_Process. The first item in the tuple
	# will keep the start of any remaining_Sequences_Data_To_Process that has
	# already been read and which contains the start of the next sequence (and
	# possibly following sequences too) to be processed. The second item in the
	# tuple is the sequence number for the sequence. We initially put an empty
	# string in it (since no data has been initially read) and set the sequence
	# number to zero. Sequence zero is considered to be anything that shows up
	# before the initial sequence header in the input and no further processing
	# will be done on it.
	#
	# Additionally the tuples in info_For_Remaining_Sequences_Data_To_Process
	# need to contain variable sized strings, it has to be shared with multiple
	# processes, and it needs to allow for code to efficiently wait for
	# something to be added to it when it's empty so a Queue appears to be the
	# best option in Python. We don't use this as much of a queue though, it
	# only ever has at most one item in it at any given time.
	info_For_Remaining_Sequences_Data_To_Process=Queue()
	info_For_Remaining_Sequences_Data_To_Process.put((b"", 0))

	# When running on a multicore system and reading input with multiple
	# sequences, this program can create multiple processes to process the
	# sequences. Passing around lots of sequence data efficiently between
	# processes could be annoying and doing so in Python is probably even more
	# annoying so instead we just have each Python process take turns reading in
	# sequence data directly from the stdin stream. Unfortunately for some
	# reason CPython reattaches the sys.stdin (and sys.__stdin__) file object to
	# the null device (but seemingly inconsistently leaves sys.stdout alone) but
	# it does leave the underlying file descriptor open. This seems a bit hacky
	# but we will use os.read() to read from the underlying file descriptor
	# instead so we record the file descriptor for stdin here. The standard file
	# streams do get inherited by processes by default currently with CPython
	# but that could maybe change in the future so it might be a good idea to
	# uncomment the second line of code below to explicitly set stdin to be
	# inheritable.
	stdin_File_Descriptor=stdin.fileno()
	#set_inheritable(stdin_File_Descriptor, True)

	# When running this program on a multicore system with input that has
	# multiple sequences, this program can start up additional processes to
	# process the sequences faster. It isn't strictly necessary but we will be
	# limiting the amount of processes used to the number of CPU cores available
	# on the system. Additionally by keeping track of how many CPU cores are
	# being used, it's possible to determine when all of the input has been
	# processed which helps out with some other problems (see last comment in
	# this code block).
	#
	# Initialize CPU_Cores_Available_For_Processing_Sequences with one less than
	# the number of CPU cores available on the system (we use one less than the
	# number of CPU cores since this main process will also be used for
	# processing sequences in just a moment). Note that using cpu_count() will
	# return the number of CPU cores online on the system but not necessarily
	# available to the program (if something like taskset is used) plus
	# cpu_count() can sometimes return incorrect numbers too. Using something
	# like len(os.sched_getaffinity(0)) on systems where it is available would
	# be more accurate but is less portable.
	CPU_Cores_Available_For_Processing_Sequences=Semaphore((cpu_count() or 1)-1)

	# These are used to let processes wait for their turns to output sequences
	# and to output them in the correct order.
	sequence_Was_Written_Condition=Condition()
	next_Sequence_Number_To_Output=Value("L", 1)


	# Start processing all the sequences.
	process_Sequences(info_For_Remaining_Sequences_Data_To_Process
	  , stdin_File_Descriptor, CPU_Cores_Available_For_Processing_Sequences
	  , sequence_Was_Written_Condition, next_Sequence_Number_To_Output)


	# It's possible for the main process to try exiting before other descendent
	# processes do. It isn't strictly a problem for the main process to exit
	# before all its descendent processes do since the descendent processes
	# inherit the standard streams of the main process and will keep them open
	# until all the sequences are processed. However it's also possible for the
	# main process to try exiting before other processes have even finished
	# starting up and that seems to sometimes cause problems with CPython (at
	# least at the moment when using the "spawn" process start method on Linux).
	# Additionally having the main process exit before all the sequences are
	# processed could potentially mess up other programs that may be monitoring
	# the program. To avoid these and potentially other issues, we wait for all
	# the processes to exit (which will be indicated by them releasing CPU cores
	# allowing this main process to acquire them all).
	for i in range(cpu_count() or 1):
		CPU_Cores_Available_For_Processing_Sequences.acquire()
