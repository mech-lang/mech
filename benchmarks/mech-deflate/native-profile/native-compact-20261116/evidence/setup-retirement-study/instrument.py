"""Apply asserted diagnostics to a local copy of the pinned runtime only."""


def once(source, old, new):
    assert source.count(old) == 1, (old, source.count(old))
    return source.replace(old, new)


def instrument(source):
    source = once(source, 'mod terminal_appends;', 'pub mod setup_probe;\nmod terminal_appends;')
    source = once(source, "pub struct Heap<'storage> {", "pub struct Heap<'storage> {\n    pub setup_probe: setup_probe::Probe,")
    source = once(source, '    ) -> Result<View, u64> {\n        if stride == 0',
                  '    ) -> Result<View, u64> {\n        let probe_started = setup_probe::stamp();\n        if stride == 0')
    source = once(source, '        let mapping = if zeroed && stride != 1 {',
                  '        let probe_payload_started = setup_probe::stamp();\n        let mapping = if zeroed && stride != 1 {')
    source = once(source, '        if data.is_null() {',
                  '        let probe_payload_ns = setup_probe::elapsed(probe_payload_started);\n        if data.is_null() {')
    source = once(source, '        Ok([data as u64, 0, capacity as u64, 1, 0, owner as u64])', '''        self.setup_probe.record(setup_probe::Event {
            op: "allocate", pointer: data as usize, bytes, stride, zeroed, mapped,
            total_ns: setup_probe::elapsed(probe_started), payload_ns: probe_payload_ns,
        });
        Ok([data as u64, 0, capacity as u64, 1, 0, owner as u64])''')
    source = once(source, '    unsafe fn release(&mut self, owner: *mut Owner) -> Result<(), u64> {',
                  '    unsafe fn release(&mut self, owner: *mut Owner) -> Result<(), u64> {\n        let probe_started = setup_probe::stamp();')
    source = once(source, '        if record.is_mapped() {\n            // SAFETY: this exact mapping', '''        let probe_stride = record.stride;
        let probe_mapped = record.is_mapped();
        let probe_payload_started = setup_probe::stamp();
        if record.is_mapped() {
            // SAFETY: this exact mapping''')
    source = once(source, '        // SAFETY: this owner belongs to the heap and its payload is retired.',
                  '        let probe_payload_ns = setup_probe::elapsed(probe_payload_started);\n        // SAFETY: this owner belongs to the heap and its payload is retired.')
    source = once(source, '        self.counters.live_payload_bytes -= bytes as u64;\n        Ok(())', '''        self.counters.live_payload_bytes -= bytes as u64;
        self.setup_probe.record(setup_probe::Event {
            op: "release", pointer: data as usize, bytes, stride: probe_stride,
            zeroed: false, mapped: probe_mapped,
            total_ns: setup_probe::elapsed(probe_started), payload_ns: probe_payload_ns,
        });
        Ok(())''')
    source = once(source, '    pub unsafe fn take_u8(&mut self, view: View) -> Result<Vec<u8>, u64> {',
                  '    pub unsafe fn take_u8(&mut self, view: View) -> Result<Vec<u8>, u64> {\n        let probe_started = setup_probe::stamp();')
    source = once(source, '        Ok(result)\n    }\n}\n\n#[cfg(all(test, target_os = "macos"))]', '''        self.setup_probe.record(setup_probe::Event {
            op: "transfer", pointer: pointer as usize, bytes: capacity as usize,
            stride: 1, zeroed: false, mapped: false,
            total_ns: setup_probe::elapsed(probe_started), payload_ns: 0,
        });
        Ok(result)
    }
}

#[cfg(all(test, target_os = "macos"))]''')
    return source
