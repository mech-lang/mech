//! Checked positional consumption for private compiler-declared wire structs.
//!
//! The Rust Deserialize implementation opts in and must support visit_seq.
//! First check the entire field-name/order/boundary envelope on the immutable
//! span; then feed each original value to the same typed field reader. This
//! removes repeated name-to-field dispatch, not eager value/type validation.
//! Reordered, partial and unknown-field images retain ordinary map decoding.
use super::*;

pub(super) const STRUCT: &str = "__mech_private_positional_image_v1";

pub(super) fn ordered(mut fields: Fields<'_>, names: &[&str]) -> Result<bool, Error> {
    for name in names {
        if fields.key_bytes()? != name.as_bytes() {
            return Ok(false);
        }
        fields.node()?;
        fields.remaining -= 1;
    }
    fields.end()?;
    Ok(true)
}

pub(super) struct Values<'a>(pub(super) Fields<'a>);
impl Values<'_> {
    pub(super) fn end(&self) -> Result<(), Error> {
        self.0.end()
    }
}
impl<'de> SeqAccess<'de> for &mut Values<'de> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if self.0.remaining == 0 {
            return Ok(None);
        }
        // Name bytes have already been checked in their exact original order.
        // Keep checked slicing here; no raw-layout cast or mutable cache exists.
        self.0.key_bytes()?;
        self.0.remaining -= 1;
        self.0.decode(seed).map(Some)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.0.remaining)
    }
}

#[cfg(test)]
mod tests;
