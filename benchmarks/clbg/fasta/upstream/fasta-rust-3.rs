// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by the Rust Project Developers
// contributed by TeXitoi
// contributed by Matt Brubeck

use std::env::args;
use std::io::{self, Write, BufWriter};

const LINE_LENGTH: usize = 60;
const BLOCK_SIZE: usize = LINE_LENGTH * 1024;
const IM: u32 = 139968;

/// Pseudo-random number generator
struct Rng(u32);

impl Rng {
    fn new() -> Self { Rng(42) }

    fn gen(&mut self, probabilities: &[(u32, u8)], block: &mut [u8]) {
        for i in block.iter_mut() {
            self.0 = (self.0 * 3877 + 29573) % IM;
            *i = probabilities.iter().find(|&&(p, _)| p >= self.0).unwrap().1;
        }
    }
}

/// From a probability distribution, generate a cumulative probability distribution.
fn cumulative_probabilities(ps: &[(char, f32)]) -> Vec<(u32, u8)> {
    ps.iter().scan(0., |sum, &(c, p)| {
        *sum += p;
        Some(((*sum * IM as f32).floor() as u32, c as u8))
    }).collect()
}

/// Output FASTA data from the provided generator function.
fn make_fasta<F, W>(n: usize, mut f: F, w: &mut W) -> io::Result<()>
    where F: FnMut(&mut [u8]), W: Write
{
    let mut block = vec![0; BLOCK_SIZE];

    // Write whole blocks.
    let num_blocks = n / BLOCK_SIZE;
    for _ in 0..num_blocks {
        f(&mut block);
        write(&block, w)?;
    }

    // Write trailing block.
    let trailing_len = n % BLOCK_SIZE;
    if trailing_len > 0 {
        f(&mut block[..trailing_len]);
        write(&block[..trailing_len], w)?;
    }
    Ok(())
}

/// Print FASTA data in 60-column lines.
#[inline(always)]
fn write<W: Write>(block: &[u8], output: &mut W) -> io::Result<()> {
    for chunk in block.chunks(LINE_LENGTH) {
        output.write_all(chunk)?;
        output.write_all(b"\n")?;
    }
    Ok(())
}

fn run(n: usize) -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout());

    // Generate a DNA sequence by copying from the given sequence.

    const ALU: &'static [u8] =
        b"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGA\
          TCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACT\
          AAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAG\
          GCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCG\
          CCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
    let mut it = ALU.iter().cloned().cycle();

    writeln!(out, ">ONE Homo sapiens alu")?;
    make_fasta(n * 2, |block| for i in block {
        *i = it.next().unwrap()
    }, &mut out)?;

    // Generate DNA sequences by weighted random selection from two alphabets.

    let p0 = cumulative_probabilities(
        &[('a', 0.27), ('c', 0.12), ('g', 0.12), ('t', 0.27), ('B', 0.02),
          ('D', 0.02), ('H', 0.02), ('K', 0.02), ('M', 0.02), ('N', 0.02),
          ('R', 0.02), ('S', 0.02), ('V', 0.02), ('W', 0.02), ('Y', 0.02)]);

    let p1 = cumulative_probabilities(
        &[('a', 0.3029549426680), ('c', 0.1979883004921),
          ('g', 0.1975473066391), ('t', 0.3015094502008)]);

    let mut rng = Rng::new();

    writeln!(out, ">TWO IUB ambiguity codes")?;
    make_fasta(n * 3, |block| rng.gen(&p0, block), &mut out)?;

    writeln!(out, ">THREE Homo sapiens frequency")?;
    make_fasta(n * 5, |block| rng.gen(&p1, block), &mut out)?;

    Ok(())
}

fn main() {
    let n = args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
    run(n).unwrap();
}
