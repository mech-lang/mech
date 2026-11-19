//! Resolve source-relative discovery hints in each physical native unit.
//! This supplies candidate boundaries, never authority to extract a region:
//! the existing checked traversal still proves the complete current CFG/ABI.
use std::ops::Range;

pub(super) struct SourceBlocks {
    source: Vec<Option<u32>>,
    auxiliary: Vec<Option<Range<u32>>>,
}

pub(super) struct Window {
    pub entry: u32,
    pub exit: u32,
    pub origin: u32,
    pub ordinal: usize,
}

impl SourceBlocks {
    pub fn new(source: Vec<Option<u32>>) -> Self {
        Self {
            auxiliary: vec![None; source.len()],
            source,
        }
    }

    pub fn record(&mut self, site: usize, first: usize, end: usize) {
        if self.source[site].is_none() {
            // Synthetic root calls/helper returns replace another unit's
            // source operation and cannot stand in for its native boundaries.
            return;
        }
        assert!(self.auxiliary[site].is_none());
        assert!(first <= end && u32::try_from(end).is_ok());
        self.auxiliary[site] = Some(first as u32..end as u32);
    }

    fn block(&self, selector: &str) -> Result<Option<u32>, &'static str> {
        let parse = |field: &str| {
            field
                .parse::<usize>()
                .map_err(|_| "invalid source-relative block selector")
        };
        if let Some(site) = selector.strip_prefix('s') {
            return self
                .source
                .get(parse(site)?)
                .copied()
                .ok_or("source block selector is outside the current Plan");
        }
        let Some((site, offset)) = selector.strip_prefix('a').and_then(|s| s.split_once('.'))
        else {
            return Err("expected s<source> or a<source>.<auxiliary offset>");
        };
        let site = parse(site)?;
        let offset = parse(offset)?;
        let Some(range) = self.auxiliary.get(site) else {
            return Err("auxiliary selector is outside the current Plan");
        };
        let Some(range) = range else {
            // A different physical unit owns this source operation.
            return Ok(None);
        };
        let offset = u32::try_from(offset).map_err(|_| "auxiliary block offset overflows")?;
        let block = range
            .start
            .checked_add(offset)
            .filter(|&block| block < range.end)
            .ok_or("source auxiliary selector exceeds the current lowering")?;
        Ok(Some(block))
    }

    pub fn windows(&self, specification: &str) -> Result<Vec<Vec<Window>>, &'static str> {
        let mut ordinal = 0;
        specification
            .split(';')
            .map(|family| {
                let mut windows = Vec::new();
                for specification in family.split(',').filter(|s| !s.is_empty()) {
                    let fields = specification.split(':').collect::<Vec<_>>();
                    let [entry, exit, origin] = fields.as_slice() else {
                        return Err("expected source entry:continuation:origin base");
                    };
                    let origin = origin.parse::<u32>().map_err(|_| "invalid source origin")?;
                    if origin as usize >= self.source.len() || ordinal >= self.source.len() {
                        return Err("source window is outside the current Plan");
                    }
                    let entry = self.block(entry)?;
                    let exit = self.block(exit)?;
                    if let (Some(entry), Some(exit)) = (entry, exit) {
                        if entry == exit {
                            return Err("source window entry equals its continuation");
                        }
                        windows.push(Window {
                            entry,
                            exit,
                            origin,
                            ordinal,
                        });
                    }
                    ordinal += 1;
                }
                Ok(windows)
            })
            .collect()
    }

    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "source_blocks": self.source,
            "auxiliary_ranges": self.auxiliary.iter().map(|range|
                range.as_ref().map(|range| [range.start, range.end])
            ).collect::<Vec<_>>(),
            "role": "discovery hints only; current checked traversal proves every selected region"
        })
    }
}

#[cfg(test)]
#[path = "source_blocks_tests.rs"]
mod tests;
