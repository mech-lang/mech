//! Observe the provider's actual retained snapshot, not process RSS estimates.
use super::*;
use std::cell::{Cell, RefCell};

#[derive(Debug)]
struct Once {
    input: RefCell<Option<Vec<u8>>>,
    acquisitions: Cell<usize>,
}
impl CliBackend for Once {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(Ok(self.input.borrow_mut().take().expect("one acquisition")))
    }
    fn stdin_text(&self) -> MResult<String> {
        panic!("raw acquisition must not be repeated as text")
    }
    fn write_stdout(&mut self, _: &str) -> MResult<()> {
        panic!("read-only test")
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        panic!("read-only test")
    }
}
fn provider(input: Vec<u8>) -> CliResourceProvider<Once> {
    CliResourceProvider::new(Once {
        input: RefCell::new(Some(input)),
        acquisitions: Cell::new(0),
    })
}
fn read(path: &str) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: "cli://cli/stdin".into(),
        path: path.into(),
        context_name: "stdin".into(),
    }
}
fn assert_bytes(value: &Value, expected: &[u8]) {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("canonical byte matrix")
    };
    let SequenceView::U8(bytes) = matrix.elements() else {
        panic!("canonical U8 storage")
    };
    assert_eq!(bytes, expected);
}

#[test]
fn decoded_snapshot_releases_raw_backing_and_preserves_both_read_orders() {
    for input in [String::new(), "a\0é\r\n".repeat(4096)] {
        for text_first in [false, true] {
            let provider = provider(input.as_bytes().to_vec());
            let before = if text_first {
                None
            } else {
                Some(provider.read(read("bytes")).unwrap())
            };
            let text = provider.read(read("text")).unwrap();
            assert!(matches!(text.data(), ValueData::String(value) if value.as_ref() == input));
            {
                let snapshot = provider.stdin_snapshot.lock().unwrap();
                assert!(
                    !matches!(snapshot.acquisition, Some(Ok(CliStdinAcquisition::Raw(_)))),
                    "successful decoded snapshot must not retain duplicate raw backing"
                );
                assert!(
                    text.shares_frozen_storage(snapshot.text.as_ref().unwrap().as_ref().unwrap())
                );
            }
            for _ in 0..3 {
                let repeated = provider.read(read("text")).unwrap();
                assert!(repeated.shares_frozen_storage(&text));
                let bytes = provider.read(read("bytes")).unwrap();
                assert_bytes(&bytes, input.as_bytes());
                if let Some(before) = &before {
                    assert!(bytes.shares_frozen_storage(before));
                }
            }
            assert_eq!(provider.backend().acquisitions.get(), 1);
            assert!(provider.backend().input.borrow().is_none());
        }
    }
}

#[test]
fn invalid_utf8_retains_raw_backing_for_later_byte_reads() {
    let input = vec![b'a', 0, 0x80, 0xff, b'\r', b'\n'];
    let provider = provider(input.clone());
    let error = provider.read(read("text")).unwrap_err().display_message();
    {
        let snapshot = provider.stdin_snapshot.lock().unwrap();
        assert!(matches!(
            snapshot.acquisition.as_ref(),
            Some(Ok(CliStdinAcquisition::Raw(bytes))) if bytes == &input
        ));
    }
    assert_bytes(&provider.read(read("bytes")).unwrap(), &input);
    assert_eq!(
        provider.read(read("text")).unwrap_err().display_message(),
        error
    );
    assert_eq!(provider.backend().acquisitions.get(), 1);
}

#[test]
fn raw_capture_transfers_backing_without_copy_in_both_read_orders() {
    for input in [String::new(), "a\0é\r\n".repeat(8192)] {
        for text_first in [false, true] {
            let raw = input.as_bytes().to_vec();
            assert_eq!(raw.capacity(), raw.len());
            let pointer = raw.as_ptr();
            let provider = provider(raw);
            let earlier_bytes = if text_first {
                None
            } else {
                Some(provider.read(read("bytes")).unwrap())
            };
            let text = provider.read(read("text")).unwrap();
            assert!(
                matches!(text.data(),ValueData::String(value) if value.as_ref() == input && value.as_ptr() == pointer)
            );
            let repeated = provider.read(read("text")).unwrap();
            assert!(text.shares_frozen_storage(&repeated));
            let bytes = provider.read(read("bytes")).unwrap();
            assert_bytes(&bytes, input.as_bytes());
            if let Some(earlier_bytes) = earlier_bytes {
                assert!(earlier_bytes.shares_frozen_storage(&bytes));
                assert_bytes(&earlier_bytes, input.as_bytes());
            }
            assert!(matches!(
                provider.stdin_snapshot.lock().unwrap().acquisition,
                Some(Ok(CliStdinAcquisition::Decoded))
            ));
            assert_eq!(provider.backend().acquisitions.get(), 1);
        }
    }
}
