// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by the Rust Project Developers
// contributed by Cristi Cobzarenco
// contributed by TeXitoi
// contributed by Matt Brubeck

extern crate rayon;

use std::cmp::min;
use std::io::{BufRead, Result, Write, stdin, stdout};
use std::mem::replace;

fn main() -> Result<()> {
    let table = build_table();
    for seq in get_sequences(&table)?.iter().rev() {
        stdout().write_all(seq)?;
    }
    Ok(())
}

/// Lookup table to find the complement of a single FASTA code.
fn build_table() -> [u8; 256] {
    let mut table = [0; 256];
    for (i, x) in table.iter_mut().enumerate() {
        *x = match i as u8 as char {
            'A' | 'a' => 'T',
            'C' | 'c' => 'G',
            'G' | 'g' => 'C',
            'T' | 't' => 'A',
            'U' | 'u' => 'A',
            'M' | 'm' => 'K',
            'R' | 'r' => 'Y',
            'W' | 'w' => 'W',
            'S' | 's' => 'S',
            'Y' | 'y' => 'R',
            'K' | 'k' => 'M',
            'V' | 'v' => 'B',
            'H' | 'h' => 'D',
            'D' | 'd' => 'H',
            'B' | 'b' => 'V',
            'N' | 'n' => 'N',
            i => i,
        } as u8;
    }
    table
}

/// Read each sequence from stdin, process it, and return it.
fn get_sequences(table: &[u8; 256]) -> Result<Vec<Vec<u8>>> {
    let stdin = stdin();
    let mut input = stdin.lock();
    let mut buf = Vec::with_capacity(16 * 1024);

    // Read the header line.
    input.read_until(b'\n', &mut buf)?;
    let start = buf.len();

    // Read sequence data.
    input.read_until(b'>', &mut buf)?;
    let end = buf.len();
    drop(input);

    if buf[end - 1] == b'>' {
        // Found the start of a new sequence. Process this one
        // and start reading the next one in parallel.
        let mut results = rayon::join(
            || reverse_complement(&mut buf[start..end - 1], &table),
            || get_sequences(table)).1?;
        results.push(buf);
        Ok(results)
    } else {
        // Reached the end of the file.
        reverse_complement(&mut buf[start..end], &table);
        Ok(vec![buf])
    }
}

/// Compute the reverse complement of one sequence.
fn reverse_complement(seq: &mut [u8], table: &[u8; 256]) {
    let len = seq.len() - 1;
    let seq = &mut seq[..len]; // Drop the last newline
    let trailing_len = len % LINE_LEN;
    let (left, right) = seq.split_at_mut(len / 2);
    reverse_complement_left_right(left, right, trailing_len, table);
}

/// Length of a normal line including the terminating \n.
const LINE_LEN: usize = 61;
/// Maximum number of bytes to process in serial.
const SEQUENTIAL_SIZE: usize = 16 * 1024;

/// Compute the reverse complement on chunks from opposite ends of a sequence.
///
/// `left` must start at the beginning of a line. If there are an odd number of
/// bytes, `right` will initially be 1 byte longer than `left`; otherwise they
/// will have equal lengths.
fn reverse_complement_left_right(mut left: &mut [u8],
                                 mut right: &mut [u8],
                                 trailing_len: usize,
                                 table: &[u8; 256]) {
    let len = left.len();
    if len <= SEQUENTIAL_SIZE {
        // Each iteration swaps one line from the start of the sequence with one
        // from the end.
        while left.len() > 0  || right.len() > 0 {
            // Get the chunk up to the newline in `right`.
            let mut a = left.split_off_left(trailing_len);
            let mut b = right.split_off_right(trailing_len);
            right.split_off_right(1); // Skip the newline in `right`.

            // If we've reached the middle of the sequence here and there is an
            // odd number of bytes remaining, the odd one will be on the right.
            if b.len() > a.len() {
                let mid = b.split_off_left(1);
                mid[0] = table[mid[0] as usize];
            }

            reverse_chunks(a, b, table);

            // Get the chunk up to the newline in `left`.
            let n = LINE_LEN - 1 - trailing_len;
            a = left.split_off_left(n);
            b = right.split_off_right(n);
            left.split_off_left(1); // Skip the newline in `left`.

            // If we've reached the middle of the sequence and there is an odd
            // number of bytes remaining, the odd one will now be on the left.
            if a.len() > b.len() {
                let mid = a.split_off_right(1);
                mid[0] = table[mid[0] as usize]
            }

            reverse_chunks(a, b, table);
        }
    } else {
        // Divide large chunks in half and fork them into two parallel tasks.
        let line_count = len / LINE_LEN;
        let mid = line_count / 2 * LINE_LEN; // Split on a whole number of lines.

        let left1 = left.split_off_left(mid);
        let right1 = right.split_off_right(mid);
        rayon::join(|| reverse_complement_left_right(left,  right,  trailing_len, table),
                    || reverse_complement_left_right(left1, right1, trailing_len, table));
    }
}

/// Compute the reverse complement for two contiguous chunks without line breaks.
fn reverse_chunks(left: &mut [u8], right: &mut [u8], table: &[u8; 256]) {
    for (x, y) in left.iter_mut().zip(right.iter_mut().rev()) {
        *y = table[replace(x, table[*y as usize]) as usize];
    }
}

/// Utilities for splitting chunks off of slices.
trait SplitOff {
    fn split_off_left(&mut self, n: usize) -> Self;
    fn split_off_right(&mut self, n: usize) -> Self;
}
impl<'a, T> SplitOff for &'a mut [T] {
    /// Split the left `n` items from self and return them as a separate slice.
    fn split_off_left(&mut self, n: usize) -> Self {
        let n = min(self.len(), n);
        let data = replace(self, &mut []);
        let (left, data) = data.split_at_mut(n);
        *self = data;
        left
    }
    /// Split the right `n` items from self and return them as a separate slice.
    fn split_off_right(&mut self, n: usize) -> Self {
        let len = self.len();
        let n = min(len, n);
        let data = replace(self, &mut []);
        let (data, right) = data.split_at_mut(len - n);
        *self = data;
        right
    }
}
