// The Computer Language Benchmarks Game
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/
//
// Contributed by Ryohei Machida
// Inspired by C++ #2 implementation Adam Kewley

extern crate memchr;
extern crate rayon;

use memchr::memchr;
use std::cmp;
use std::fs::File;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::io::FromRawFd;

const READ_SIZE: usize = 1 << 16;

/// Length of a normal line including the terminating \n.
const LINE_LEN: usize = 60;
/// Maximum number of rows to process in serial.
const BLOCK_ROWS: usize = 4096;

#[rustfmt::skip]
static KNUCLEOTIDE_MAPPING: &[u8; 256] = b"\
    \0\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f\
    \x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\
    \x20!\"#$%&'()*+,-./\
    0123456789:;<=>?\
    @TVGHEFCDIJMLKNO\
    PQYSAABWXRZ[\\]^_\
    `TVGHefCDijMlKNo\
    pqYSAABWxRz{|}~\x7f\
    \x80\x81\x82\x83\x84\x85\x86\x87\x88\x89\x8a\x8b\x8c\x8d\x8e\x8f\
    \x90\x91\x92\x93\x94\x95\x96\x97\x98\x99\x9a\x9b\x9c\x9d\x9e\x9f\
    \xa0\xa1\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xab\xac\xad\xae\xaf\
    \xb0\xb1\xb2\xb3\xb4\xb5\xb6\xb7\xb8\xb9\xba\xbb\xbc\xbd\xbe\xbf\
    \xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xcb\xcc\xcd\xce\xcf\
    \xd0\xd1\xd2\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xdb\xdc\xdd\xde\xdf\
    \xe0\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xeb\xec\xed\xee\xef\
    \xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xfb\xfc\xfd\xfe\xff";

#[cfg(target_feature = "ssse3")]
mod ssse3 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    use super::KNUCLEOTIDE_MAPPING;

    /// reverse bytes and complement each byte
    #[rustfmt::skip]
    unsafe fn reverse_chunks_simd(mut v: __m128i) -> __m128i {
        v = _mm_shuffle_epi8(
            v,
            _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15),
        );
        v = _mm_and_si128(v, _mm_set1_epi8(0x1f));

        let lt16_els = _mm_add_epi8(v, _mm_set1_epi8(0x70));
        let lt16_lut = _mm_set_epi8(
            0,          78 /* N */, 75 /* K */, 0,
            77 /* M */, 10,         0,          68 /* D */,
            67 /* C */, 0,          0,          72 /* H */,
            71 /* G */, 86 /* V */, 84 /* T */, 0,
        );
        let lt16_vals = _mm_shuffle_epi8(lt16_lut, lt16_els);

        let g16_els = _mm_sub_epi8(v, _mm_set1_epi8(0x10));
        let g16_lut = _mm_set_epi8(
            0,          0,          0,          0,
            0,          0,          82 /* R */, 0,
            87 /* W */, 66 /* B */, 65 /* A */, 65 /* A */,
            83 /* S */, 89 /* Y */, 0,          0,
        );
        let g16_vals = _mm_shuffle_epi8(g16_lut, g16_els);

        return _mm_or_si128(lt16_vals, g16_vals);
    }

    pub fn reverse_chunks(left: &mut [u8], right: &mut [u8]) {
        debug_assert_eq!(left.len(), right.len());

        unsafe {
            let mut len = left.len();
            let mut pl = left.as_mut_ptr();
            let mut pr = right.as_mut_ptr().add(right.len());

            while len >= 16 {
                pr = pr.sub(16);
                let l = _mm_lddqu_si128(pl as _);
                let r = _mm_lddqu_si128(pr as _);
                _mm_storeu_si128(pr as _, reverse_chunks_simd(l));
                _mm_storeu_si128(pl as _, reverse_chunks_simd(r));
                pl = pl.add(16);

                len -= 16;
            }

            for _ in 0..len {
                pr = pr.sub(1);
                let l = *pl;
                let r = *pr;
                *pr = KNUCLEOTIDE_MAPPING[l as usize];
                *pl = KNUCLEOTIDE_MAPPING[r as usize];
                pl = pl.add(1);
            }
        }
    }
}

#[cfg(target_feature = "ssse3")]
use ssse3::*;

#[cfg(not(target_feature = "ssse3"))]
mod fallback {
    use super::KNUCLEOTIDE_MAPPING;

    pub fn reverse_chunks(left: &mut [u8], right: &mut [u8]) {
        for (pl, pr) in left.iter_mut().zip(right.iter_mut().rev()) {
            let l = *pl;
            let r = *pr;
            *pr = KNUCLEOTIDE_MAPPING[l as usize];
            *pl = KNUCLEOTIDE_MAPPING[r as usize];
        }
    }
}

#[cfg(not(target_feature = "ssse3"))]
use fallback::*;

struct Sequence<'a> {
    buf: &'a mut [u8],
    content_offset: usize,
}

impl<'a> Sequence<'a> {
    fn from_slice(data: &'a mut [u8]) -> Option<Sequence<'a>> {
        match memchr(b'\n', data) {
            Some(pos) => Some(Sequence { buf: data, content_offset: pos + 1 }),
            None => None,
        }
    }

    fn as_slice(&self) -> &[u8] {
        self.buf
    }

    fn get_content_mut(&mut self) -> &mut [u8] {
        // remove header and last character (\n)
        let end = self.buf.len() - 1;
        &mut self.buf[self.content_offset..end]
    }

    /// reverse and complement the whole sequence
    fn reverse_complement(&mut self) {
        let mut content = self.get_content_mut();
        let block_bytes = BLOCK_ROWS * (LINE_LEN + 1);
        let trailing_len = content.len() % (LINE_LEN + 1);

        rayon::scope(|scope| {
            while content.len() >= block_bytes * 2 {
                let (left, tmp) = content.split_at_mut(block_bytes);
                let (inner, right) = tmp.split_at_mut(tmp.len() - block_bytes);
                content = inner;

                scope.spawn(move |_| {
                    reverse_complement_left_right(left, right, trailing_len)
                });
            }

            let n = content.len() / 2;
            let (left, right) = content.split_at_mut(n);
            scope.spawn(move |_| {
                reverse_complement_left_right(left, right, trailing_len)
            });
        });
    }
}

fn reverse_complement_left_right(
    mut left: &mut [u8],
    mut right: &mut [u8],
    trailing_len: usize,
) {
    debug_assert!(left.len() <= right.len());
    debug_assert!(right.len() <= left.len() + 1);

    while left.len() >= trailing_len {
        let (n, m) = (trailing_len, right.len() - trailing_len);
        let (a, left_) = left.split_at_mut(n);
        let (right_, b) = right.split_at_mut(m);
        left = left_;
        right = &mut right_[..m - 1];

        reverse_chunks(a, b);

        let n = LINE_LEN - trailing_len;
        if right.len() <= n {
            break;
        }
        let m = right.len() - n;
        let (a, left_) = left.split_at_mut(n);
        let (right_, b) = right.split_at_mut(m);
        left = &mut left_[1..];
        right = right_;

        reverse_chunks(a, b);
    }

    let n = cmp::min(left.len(), right.len());
    let m = right.len() - n;
    reverse_chunks(&mut left[..n], &mut right[m..]);

    // character at the middle of sequence
    let mid = if left.len() > right.len() {
        left.last_mut().unwrap()
    } else if right.len() > left.len() {
        right.first_mut().unwrap()
    } else {
        return;
    };

    *mid = KNUCLEOTIDE_MAPPING[*mid as usize];
}

/// Scan stdin directly into the growing buffer
struct SequenceReader<R: Read> {
    inner: R,
    buf: Vec<u8>,
    next_pos: usize,
    eof_reached: bool,
}

impl<R: Read> SequenceReader<R> {
    fn new(inner: R) -> Self {
        SequenceReader {
            inner,
            buf: Vec::new(),
            next_pos: 0,
            eof_reached: false,
        }
    }

    fn next(&mut self) -> Option<io::Result<Sequence<'_>>> {
        // scan in buffer first
        if !self.buf.is_empty() {
            if self.next_pos >= self.buf.len() {
                return None;
            }

            // remove current sequence (first `self.next_pos` bytes)
            self.buf.copy_within(self.next_pos.., 0);
            self.buf.truncate(self.buf.len() - self.next_pos);

            // find next header in buffer
            let next_header_pos = match memchr(b'>', &self.buf[1..]) {
                Some(pos) => Some(pos + 1),
                None if self.eof_reached => Some(self.buf.len()),
                None => None,
            };

            if let Some(pos) = next_header_pos {
                self.next_pos = pos;
                return Some(Ok(
                    Sequence::from_slice(&mut self.buf[..pos]).unwrap()
                ));
            }
        }

        self.buf.reserve(READ_SIZE);

        loop {
            let old_len = self.buf.len();

            // read at most READ_SIZE bytes
            let mut reader = self.inner.by_ref().take(READ_SIZE as u64);
            let read_len = match reader.read_to_end(&mut self.buf) {
                Ok(n) => n,
                Err(e) => {
                    self.eof_reached = true;
                    return Some(Err(e));
                }
            };

            self.eof_reached = read_len != READ_SIZE;

            // find next header
            let offset = cmp::min(cmp::max(old_len, 1), self.buf.len());
            self.next_pos = match memchr(b'>', &self.buf[offset..]) {
                None if !self.eof_reached => continue,
                Some(pos) => offset + pos,
                None => self.buf.len(),
            };

            // if next header is found, return the slice to current buffer
            return Some(Ok(Sequence::from_slice(
                &mut self.buf[..self.next_pos],
            )
            .unwrap()));
        }
    }
}

fn main() -> io::Result<()> {
    // Use unbuffered stdin and stdout on unix platform
    #[cfg(unix)]
    let stdin = unsafe { File::from_raw_fd(0) };
    #[cfg(unix)]
    let mut stdout = unsafe { File::from_raw_fd(1) };

    #[cfg(not(unix))]
    let stdin = io::stdin();
    #[cfg(not(unix))]
    let stdin = stdin.lock();
    #[cfg(not(unix))]
    let mut stdout = io::stdout();

    let mut reader = SequenceReader::new(stdin);

    while let Some(seq) = reader.next() {
        let mut seq = seq?;
        seq.reverse_complement();
        stdout.write_all(seq.as_slice())?;
    }

    Ok(())
}
