use super::*;
use std::cell::Cell;
use std::io::Cursor;

struct ShortReads<'a> {
    bytes: &'a [u8],
    position: usize,
    step: usize,
    interrupt: bool,
    calls: usize,
    error_at: Option<usize>,
}
impl<'a> ShortReads<'a> {
    fn new(bytes: &'a [u8], step: usize) -> Self {
        Self {
            bytes,
            position: 0,
            step,
            interrupt: false,
            calls: 0,
            error_at: None,
        }
    }
}
impl Read for ShortReads<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.interrupt && self.calls % 2 == 1 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.error_at == Some(self.position) {
            return Err(io::Error::other("capture fixture I/O failure"));
        }
        let count = output
            .len()
            .min(self.step)
            .min(self.bytes.len() - self.position)
            .min(self.error_at.map_or(usize::MAX, |at| at - self.position));
        output[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

#[test]
fn every_unicode_split_short_read_and_interruption_preserves_exact_text() {
    for input in ["", "a\0b\r\n\nlast\r", "éλ漢字🦎", "x\r\né🦎\n"] {
        for chunk in 1..=11 {
            for step in 1..=7 {
                let memory = ManagedMemoryBudget::new(64);
                let pump =
                    RuntimeTextCaptureBuffer::prepare(input.len() as u64, chunk, &memory).unwrap();
                let capacity = pump.retained_capacity_bytes();
                let mut reader = ShortReads::new(input.as_bytes(), step);
                reader.interrupt = true;
                let mut text = String::new();
                let summary = pump
                    .capture(&mut reader, |fragment| {
                        assert_eq!(memory.used_bytes(), capacity);
                        assert!(fragment.len() <= chunk + UTF8_CARRY);
                        text.push_str(fragment);
                        Ok::<_, ()>(())
                    })
                    .unwrap();
                assert_eq!(text, input);
                assert_eq!(summary.input_bytes(), input.len() as u64);
                assert_eq!(reader.position, input.len());
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn incremental_utf8_errors_match_whole_input_validation() {
    let mut inputs = (0..=255u8)
        .map(|byte| vec![b'o', b'k', byte])
        .collect::<Vec<_>>();
    inputs.extend([
        vec![0xe0, 0x80, 0x80],       // overlong
        vec![0xed, 0xa0, 0x80],       // surrogate
        vec![0xf4, 0x90, 0x80, 0x80], // above Unicode
        vec![b'a', 0xf0, 0x9f, 0xa6], // truncated
        vec![b'a', 0xc3],
        vec![b'a', 0xe6, 0xbc],
        "oké🦎".as_bytes().to_vec(),
    ]);
    for input in inputs {
        for chunk in 1..=6 {
            for step in 1..=3 {
                let memory = ManagedMemoryBudget::new(32);
                let pump =
                    RuntimeTextCaptureBuffer::prepare(input.len() as u64, chunk, &memory).unwrap();
                let mut reader = ShortReads::new(&input, step);
                let mut received = String::new();
                let actual = pump.capture(&mut reader, |fragment| {
                    received.push_str(fragment);
                    Ok::<_, ()>(())
                });
                match (std::str::from_utf8(&input), actual) {
                    (Ok(expected), Ok(summary)) => {
                        assert_eq!(received, expected);
                        assert_eq!(summary.input_bytes(), input.len() as u64);
                    }
                    (
                        Err(expected),
                        Err(RuntimeTextCaptureError::InvalidUtf8 {
                            valid_up_to,
                            error_len,
                        }),
                    ) => {
                        assert_eq!(valid_up_to, expected.valid_up_to() as u64, "{input:?}");
                        assert_eq!(error_len, expected.error_len(), "{input:?}");
                    }
                    pair => panic!("{input:?}, chunk={chunk}, step={step}: {pair:?}"),
                }
                assert_eq!(reader.position, input.len());
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn exact_input_cap_uses_one_probe_and_never_reads_later_io() {
    for chunk in 1..=9 {
        let memory = ManagedMemoryBudget::new(32);
        let pump = RuntimeTextCaptureBuffer::prepare(3, chunk, &memory).unwrap();
        let mut reader = ShortReads::new(b"abcdef", 2);
        reader.error_at = Some(5);
        let result = pump.capture(&mut reader, |_| Ok::<_, ()>(()));
        assert!(matches!(
            result,
            Err(RuntimeTextCaptureError::InputLimit { limit: 3 })
        ));
        assert_eq!(reader.position, 4);
        assert_eq!(memory.used_bytes(), 0);
        let mut reader = Cursor::new(b"abc");
        let pump = RuntimeTextCaptureBuffer::prepare(3, chunk, &memory).unwrap();
        assert_eq!(
            pump.capture(&mut reader, |_| Ok::<_, ()>(()))
                .unwrap()
                .input_bytes(),
            3
        );
        let pump = RuntimeTextCaptureBuffer::prepare(0, chunk, &memory).unwrap();
        assert_eq!(
            pump.capture(Cursor::new(b""), |_| Ok::<_, ()>(()))
                .unwrap()
                .input_bytes(),
            0
        );
        let mut reader = Cursor::new(b"xmore");
        let pump = RuntimeTextCaptureBuffer::prepare(0, chunk, &memory).unwrap();
        assert!(matches!(
            pump.capture(&mut reader, |_| Ok::<_, ()>(())),
            Err(RuntimeTextCaptureError::InputLimit { limit: 0 })
        ));
        assert_eq!(reader.position(), 1);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn capture_errors_precede_bad_utf8_and_deferred_consumer_errors() {
    let memory = ManagedMemoryBudget::new(32);
    for bad_utf8 in [false, true] {
        let input: &[u8] = if bad_utf8 {
            b"a\xffrest"
        } else {
            b"valid text"
        };
        for io_failure in [false, true] {
            let pump = RuntimeTextCaptureBuffer::prepare(input.len() as u64, 1, &memory).unwrap();
            let mut reader = ShortReads::new(input, 1);
            if io_failure {
                reader.error_at = Some(input.len());
            }
            let calls = Cell::new(0);
            let actual = pump.capture(&mut reader, |_| {
                calls.set(calls.get() + 1);
                Err("native computation failure")
            });
            match actual {
                Err(RuntimeTextCaptureError::Acquisition(error)) if io_failure => {
                    assert_eq!(error.to_string(), "capture fixture I/O failure");
                }
                Err(RuntimeTextCaptureError::InvalidUtf8 {
                    valid_up_to: 1,
                    error_len: Some(1),
                }) if bad_utf8 => {}
                Err(RuntimeTextCaptureError::Consumer("native computation failure"))
                    if !bad_utf8 && !io_failure => {}
                other => panic!("bad_utf8={bad_utf8}, io_failure={io_failure}: {other:?}"),
            }
            assert_eq!(calls.get(), 1);
            assert_eq!(reader.position, input.len());
            assert_eq!(memory.used_bytes(), 0);
        }
    }
    let mut reader = Cursor::new(b"a\xffrest");
    let pump = RuntimeTextCaptureBuffer::prepare(2, 1, &memory).unwrap();
    let actual = pump.capture(&mut reader, |_| Err("native computation failure"));
    assert!(matches!(
        actual,
        Err(RuntimeTextCaptureError::InputLimit { limit: 2 })
    ));
    assert_eq!(reader.position(), 3);
}

#[test]
fn consumer_panics_are_deferred_until_eof_and_keep_the_original_payload() {
    let memory = ManagedMemoryBudget::new(32);
    let mut reader = Cursor::new(b"all authorized input");
    let pump = RuntimeTextCaptureBuffer::prepare(20, 1, &memory).unwrap();
    let calls = Cell::new(0);
    let caught = catch_unwind(AssertUnwindSafe(|| {
        pump.capture::<_, ()>(&mut reader, |_| {
            calls.set(calls.get() + 1);
            std::panic::panic_any(0x1234u32);
        })
    }));
    assert_eq!(*caught.unwrap_err().downcast::<u32>().unwrap(), 0x1234);
    assert_eq!(calls.get(), 1);
    assert_eq!(reader.position(), 20);
    assert_eq!(memory.used_bytes(), 0);
    for (input, io_failure) in [(b"a\xff".as_slice(), false), (b"valid".as_slice(), true)] {
        let mut reader = ShortReads::new(input, 1);
        if io_failure {
            reader.error_at = Some(input.len());
        }
        let pump = RuntimeTextCaptureBuffer::prepare(input.len() as u64, 1, &memory).unwrap();
        let caught = catch_unwind(AssertUnwindSafe(|| {
            pump.capture::<_, ()>(&mut reader, |_| {
                std::panic::panic_any(0x5678u32);
            })
        }));
        let error = caught
            .expect("capture error must take precedence over deferred panic")
            .unwrap_err();
        if io_failure {
            assert!(matches!(error, RuntimeTextCaptureError::Acquisition(_)));
        } else {
            assert!(matches!(error, RuntimeTextCaptureError::InvalidUtf8 { .. }));
        }
        assert_eq!(reader.position, input.len());
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn preparation_is_funded_before_allocation_or_any_read_and_shares_the_account() {
    assert!(matches!(
        RuntimeTextCaptureBuffer::prepare(u64::MAX, 1, &ManagedMemoryBudget::new(0)),
        Err(RuntimeTextCaptureSetupError::InvalidLimit)
    ));
    assert!(matches!(
        RuntimeTextCaptureBuffer::prepare(1, 0, &ManagedMemoryBudget::new(0)),
        Err(RuntimeTextCaptureSetupError::InvalidChunkSize)
    ));
    let required = 131;
    let memory = ManagedMemoryBudget::new(required - 1);
    assert!(matches!(
        RuntimeTextCaptureBuffer::prepare(4096, 128, &memory),
        Err(RuntimeTextCaptureSetupError::Memory(
            MemoryRuntimeError::BudgetExceeded { .. }
        ))
    ));
    assert_eq!(memory.used_bytes(), 0);
    let memory = ManagedMemoryBudget::new(required);
    let shared = memory.clone();
    let pump = RuntimeTextCaptureBuffer::prepare(4096, 128, &memory).unwrap();
    assert_eq!(pump.retained_capacity_bytes(), required);
    assert_eq!(shared.used_bytes(), required);
    assert!(shared.reserve_capacity(1).is_err());
    let mut reader = Cursor::new(b"small");
    pump.capture(&mut reader, |_| Ok::<_, ()>(())).unwrap();
    assert_eq!(shared.used_bytes(), 0);
    let pump = RuntimeTextCaptureBuffer::prepare(0, usize::MAX, &shared).unwrap();
    assert_eq!(pump.retained_capacity_bytes(), 4);
    drop(pump);
    assert_eq!(shared.used_bytes(), 0);
}

#[test]
fn large_generated_input_keeps_only_fixed_scratch_and_does_not_collect_text() {
    let chunk = 16 * 1024;
    let length = 16 * 1024 * 1024;
    let memory = ManagedMemoryBudget::new(chunk + UTF8_CARRY as u64);
    let pump = RuntimeTextCaptureBuffer::prepare(length, chunk as usize, &memory).unwrap();
    let mut received = 0;
    let summary = pump
        .capture(io::repeat(b'a').take(length), |text| {
            assert_eq!(memory.used_bytes(), chunk + UTF8_CARRY as u64);
            assert!(text.bytes().all(|byte| byte == b'a'));
            received += text.len() as u64;
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(received, length);
    assert_eq!(summary.input_bytes(), length);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn an_invalid_read_extent_is_not_trusted_as_initialized_input() {
    struct Liar;
    impl Read for Liar {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            Ok(output.len() + 1)
        }
    }
    let memory = ManagedMemoryBudget::new(8);
    let pump = RuntimeTextCaptureBuffer::prepare(1, 1, &memory).unwrap();
    assert!(
        matches!(pump.capture(Liar, |_| Ok::<_, ()>(())), Err(RuntimeTextCaptureError::Acquisition(error)) if error.kind()==io::ErrorKind::InvalidData)
    );
    assert_eq!(memory.used_bytes(), 0);
}
