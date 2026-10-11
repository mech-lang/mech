# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Ruben Vorderman

"""
This program reads a FASTA file from stdin and outputs a FASTA file to stdout
with all the sequences reverse complemented.
"""
# The rationale for this program is described in a comment below.

import sys
from typing import BinaryIO, Iterator, List, Tuple

# BLOCK_SIZE: larger means less blocks and therefore less Python overhead.
# smaller means less cache misses. There is a tradeoff.
# The optimal setting on my test machine is:
BLOCK_SIZE = 32 * 1024


def parse_fasta(inp: BinaryIO) -> Iterator[Tuple[bytes, List[bytes]]]:
    block_size = BLOCK_SIZE
    name_index = 0
    block = inp.read(block_size)
    while True:
        name_end = block.find(b"\n", name_index)
        if name_end == -1:  # name runs past the end of block
            name_part = block[name_index:]
            block = inp.read(block_size)
            if block == b"":
                raise EOFError("truncated FASTA file")
            name_end = block.find(b"\n", name_index)
            name = name_part + block[:name_end]
        else:
            name = block[name_index: name_end]
        seq_parts = []
        while True:  # gather all blocks until the next sequence
            name_index = block.find(b">", name_end)
            if name_index != -1:
                seq_parts.append(block[name_end:name_index])
                yield name, seq_parts
                break
            seq_parts.append(block[name_end:])
            block = inp.read(block_size)
            if block == b"":
                yield name, seq_parts
                return
            name_end = 0


def reverse_complement(inp: BinaryIO, outp: BinaryIO):
    letters = b"ACGTUMRYKVHDB"
    complmn = b"TGCAAKYRMBDHV"
    translate_table = bytes.maketrans(
        letters + letters.lower(),
        complmn + complmn,
    )
    line_length = 60
    for name, sequence_parts in parse_fasta(inp):
        outp.write(name)
        outp.write(b"\n")
        last_line_length = 0
        for part in reversed(sequence_parts):
            translated = part.translate(translate_table, b'\n')
            reverse = translated[::-1]
            offset = line_length - last_line_length
            outp.write(reverse[:offset])  # Fill out remaining bytes on the line.
            outp.write(b"\n")
            fasta_lines = [reverse[i:i + line_length]
                           for i in range(offset, len(reverse), line_length)]
            last_line_length = len(fasta_lines[-1])
            outp.write(b"\n".join(fasta_lines))
        outp.write(b"\n")  # Terminate entire sequence with final newline.
        # This del statement just before a new sequence is read ensures there
        # is only one sequence in memory at the time.
        del sequence_parts
    outp.flush()


if __name__ == "__main__":
    reverse_complement(sys.stdin.buffer, sys.stdout.buffer)


# The most common Python implementation, CPython, is written in C. Therefore
# many builtin methods and functions are implemented in C. The trick for
# writing fast Python is therefore to leverage as much of this C power as
# possible.
# For this particular problem we need to do the following things
# 1. Parse FASTA. The structure is fairly simple. A '>' starts a sequence name,
#    the first '\n' terminates the sequence name. And everything between that
#    and the next '>' is a sequence.
#    We can diminish Python overhead by reading entire 64KB blocks from the
#    input rather than lines and use the 'find' method on bytes objects
#    (implemented in C!) to find where the sequences start and begin.
# 2. Complement the sequence. in C this would best be accomplished by a
#    lookup table. A 256-characters long array. So that every character's
#    ordinal is equal to the index of the array. So 'A' is index 65. At index
#    65 of the lookup table we can put the reverse complement of 'A': 'T'.
#    Python has a convenience function that can make such a complement table
#    bytes.maketrans(). It can also use the table using the 'translate' method
#    on the bytes object. This is all implemented in C so this way we can
#    easily leverage some C performance.
# 3. Reverse the sequence. Using the slicing operator we can do
#    'sequence[::-1]'. This creates a new object, where the original sequence
#    is copied into but stepping in reverse. Because this creates a new object
#    rather than reversing in place, Python does not have to worry about
#    overwriting memory it still needs to read from. This is therefore faster
#    than using the 'reverse()' method on bytearray objects.
# 4. Removing the old formatting in the form of newlines which are still
#    present in the sequence. We can already tackle this at step 2, by using
#    the translate method on the bytes object to also remove the '\n'
#    characters. I didn't know this was possible until I saw it in the other
#    submitted programs. The python standard library has many gems!
# 5. Reformatting the sequence for printing. Unfortunately I have not found
#    a fast method for this, so this program will use the slicing operator
#    to create a new object for *every single line* and then join them using
#    the 'join()' method which allows putting newlines in between. It would be
#    much faster if there were a method to copy the sequence to a new buffer
#    with newlines in between, but there is no standard library that does this
#    as far as I know. This is the slowest part of the program taking roughly
#    40% of the execution time creating all this individual objects. When this
#    becomes a problem in the real-world you can write a custom C extension
#    (either by hand or using Cython) that tackles this problem. But in this
#    benchmark game it would of course be cheating to use C.
#    I have tried other methods, including working with a mutable bytesarray
#    and working with ctypes.memmove, but simple b"\n".join(fasta_lines) is
#    both the most simple and efficient.
#
# The above described implementation has one disadvantage. Sequences
# are treated as large strings. But these sequences can be several megabytes.
# This has bad cache locality. Cache is the memory close to the CPU and is
# limited in size (usually in the high KB low MB range). This means
# that a string of several tens of megabytes does not fit in cache. When
# the program starts on it, the start of the string is loaded in the cache.
# During the run each part of the string is loaded sequentially in cache.
# When it reaches the end, most of the string is already evicted from the
# cache. When we do a new operation on the string, the parts have to be loaded
# again. The CPU will spend a lot of time waiting.
#
# This is solved by saving the sequence as a list of strings rather than one
# big string. So instead of reversing ABCDEFGHI -> IHGFEDCBA we instead use
# [ABC, DEF, GHI]. Then reverse the list [GHI, DEF, ABC] and then revert
# individual components [IHG, FED, CBA]. The advantage is that this way we can
# complement, reverse and format the same block while it is still in the cache
# and then write it to the output and forget about it. This does wonders for
# speed because of the cache locality and is good for memory usage, because
# we don't need to save results in very large buffers. The only disadvantage
# of this method is that writing to the output becomes slightly more difficult.
# We have to remember the length of the last line we have written in order to
# put the newline at the desired line length. This is however fairly easy to
# implement.
