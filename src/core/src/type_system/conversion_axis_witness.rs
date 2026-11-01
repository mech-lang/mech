//! Conversion-only present-data Matrix axes survive cardinality refresh.
use super::{DimensionExpr, TypeResolutionError, Vec, invalid_rule};

pub(super) struct ConversionAxisWitnesses<'a, 's> {
    seed: &'s [u64],
    axes: Vec<(&'a DimensionExpr, u64)>,
}

impl<'a, 's> ConversionAxisWitnesses<'a, 's> {
    pub(super) fn new(seed: &'s [u64]) -> Self {
        Self {
            seed,
            axes: Vec::new(),
        }
    }

    pub(super) fn record(
        &mut self,
        dimensions: &'a [DimensionExpr],
    ) -> Result<(), TypeResolutionError> {
        for dimension in dimensions {
            // A schema dimension can occur in arbitrarily many data elements.
            // Retain it once, not one allocation/witness per present element.
            if self
                .axes
                .iter()
                .any(|(old, _)| core::ptr::eq(*old, dimension))
            {
                continue;
            }
            let expected = crate::evaluate_dimension(dimension, self.seed)
                .map_err(TypeResolutionError::semantic)?;
            self.axes.push((dimension, expected));
        }
        Ok(())
    }

    pub(super) fn validate(self, values: &[u64]) -> Result<(), TypeResolutionError> {
        for (dimension, expected) in self.axes {
            let actual = crate::evaluate_dimension(dimension, values)
                .map_err(TypeResolutionError::semantic)?;
            if actual != expected {
                return Err(invalid_rule(
                    "conversion cardinality changes an already witnessed Matrix axis",
                ));
            }
        }
        Ok(())
    }
}
