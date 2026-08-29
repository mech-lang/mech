use super::captures::{increment, read_bool, write_bool, write_selected_arm};
use super::{ActivationPatternCapture, GuardFinalize, ReactiveBindingSink};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, GenericError, MechError, MechFunctionCompiler, Register};
use crate::{
    CompiledPattern, FunctionStatePort, MResult, MechFunctionImpl, PatternBindingSink,
    ReactiveDependencyKind, ReactiveDependencyScope, ReactiveSolveStatus, ValueCell,
    match_compiled_pattern_with_values,
};

fn primary(cell: &ValueCell) -> Option<FunctionStatePort<'_>> {
    Some(FunctionStatePort::from_cell(cell))
}

fn retained<'a>(cells: &[&'a ValueCell]) -> MResult<Option<Vec<FunctionStatePort<'a>>>> {
    Ok(Some(
        cells
            .iter()
            .map(|cell| FunctionStatePort::from_cell(cell))
            .collect(),
    ))
}

pub(super) struct ScopePulse {
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for ScopePulse {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        increment(&self.out)?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        retained(&[&self.out])
    }
    fn reactive_dependency_scopes(&self, _: usize) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Root])
    }
    fn to_string(&self) -> String {
        "ActivationPatternScopePulse".into()
    }
}

pub(super) struct Matcher {
    pub(super) pattern: CompiledPattern,
    pub(super) trigger: ValueCell,
    pub(super) expression_values: Vec<ValueCell>,
    pub(super) captures: Vec<ActivationPatternCapture>,
    pub(super) matched: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for Matcher {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let pattern_match = match_compiled_pattern_with_values(
            &self.pattern,
            &self.trigger,
            &self.expression_values,
        )?;
        ReactiveBindingSink {
            captures: &self.captures,
        }
        .commit(&pattern_match)?;
        write_bool(&self.matched, pattern_match.matched)?;
        increment(&self.out)?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        Some(
            std::iter::once(FunctionStatePort::from_cell(&self.out))
                .chain(
                    self.captures
                        .iter()
                        .map(|capture| FunctionStatePort::from_cell(&capture.proposed)),
                )
                .collect(),
        )
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        std::iter::once(self.out.clone())
            .chain(self.captures.iter().map(|capture| capture.proposed.clone()))
            .collect()
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(
            std::iter::once(FunctionStatePort::from_cell(&self.out))
                .chain(std::iter::once(FunctionStatePort::from_cell(&self.matched)))
                .chain(
                    self.captures
                        .iter()
                        .map(|capture| FunctionStatePort::from_cell(&capture.proposed)),
                )
                .collect(),
        ))
    }
    fn reactive_dependency_kinds(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyKind>> {
        let mut kinds = vec![ReactiveDependencyKind::Sampled; argument_count];
        if let Some(scope_pulse) = kinds.first_mut() {
            *scope_pulse = ReactiveDependencyKind::Reactive;
        }
        Some(kinds)
    }
    fn to_string(&self) -> String {
        "ActivationPatternMatcher".into()
    }
}

pub(super) struct Finalize {
    pub(super) matched: ValueCell,
    pub(super) eligible: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for Finalize {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        write_bool(&self.eligible, read_bool(&self.matched)?)?;
        increment(&self.out)?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        retained(&[&self.out, &self.eligible])
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmFinalize".into()
    }
}

pub(super) struct MatchGate {
    pub(super) matched: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for MatchGate {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if read_bool(&self.matched)? {
            increment(&self.out)?;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        retained(&[&self.out])
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardMatchGate".into()
    }
}

pub(super) struct UnmatchedFinalize {
    pub(super) matched: ValueCell,
    pub(super) eligible: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for UnmatchedFinalize {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if read_bool(&self.matched)? {
            Ok(ReactiveSolveStatus::Unchanged)
        } else {
            write_bool(&self.eligible, false)?;
            increment(&self.out)?;
            Ok(ReactiveSolveStatus::Changed)
        }
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        retained(&[&self.out, &self.eligible])
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardUnmatchedFinalize".into()
    }
}

pub(super) struct Select {
    pub(super) eligible: Vec<ValueCell>,
    pub(super) selected: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for Select {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        let selected = self
            .eligible
            .iter()
            .position(|cell| read_bool(cell).unwrap_or(false))
            .unwrap_or(usize::MAX);
        write_selected_arm(&self.selected, selected)?;
        increment(&self.out)?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        primary(&self.out)
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        retained(&[&self.out, &self.selected])
    }
    fn to_string(&self) -> String {
        "ActivationPatternSelectArm".into()
    }
}

#[cfg(feature = "semantic-compiler")]
macro_rules! interpreter_only {
    ($t:ty) => {
        impl MechFunctionCompiler for $t {
            fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Err(MechError::new(
                    GenericError {
                        msg: "Activation pattern dispatch is interpreter-only.".into(),
                    },
                    None,
                ))
            }
        }
    };
}
#[cfg(feature = "semantic-compiler")]
interpreter_only!(ScopePulse);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(Matcher);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(Finalize);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(MatchGate);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(UnmatchedFinalize);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(GuardFinalize);
#[cfg(feature = "semantic-compiler")]
interpreter_only!(Select);
