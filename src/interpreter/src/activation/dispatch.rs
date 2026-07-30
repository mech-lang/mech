use super::captures::transaction_bool_state;
use super::{ActivationPatternCapture, GuardFinalize, ReactiveBindingSink};
#[cfg(feature = "compiler")]
use crate::{
    BytecodeCompilerContext, GenericError, MechError, MechFunctionCompiler, Register,
};
use crate::{
    CompiledPattern, MResult, MechFunctionImpl, PatternBindingSink, ReactiveDependencyKind,
    ReactiveDependencyScope, ReactiveSolveStatus, Ref, Value, match_compiled_pattern_with_values,
};

pub(super) struct ScopePulse {
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for ScopePulse {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_dependency_scopes(&self, _: usize) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Root])
    }
    fn to_string(&self) -> String {
        "ActivationPatternScopePulse".into()
    }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
  }
}
pub(super) struct Matcher {
    pub(super) pattern: CompiledPattern,
    pub(super) trigger: Value,
    pub(super) expression_values: Vec<Value>,
    pub(super) captures: Vec<ActivationPatternCapture>,
    pub(super) matched: Ref<bool>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for Matcher {
    fn solve(&self) {}
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
        *self.matched.borrow_mut() = pattern_match.matched;
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        let mut outputs = vec![self.out()];
        outputs.extend(self.captures.iter().map(|capture| capture.proposed.clone()));
        outputs
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.matched)?);
        Ok(values)
    }
    fn reactive_dependency_kinds(&self, argument_count: usize) -> Option<Vec<ReactiveDependencyKind>> {
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
    pub(super) matched: Ref<bool>,
    pub(super) eligible: Ref<bool>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for Finalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.eligible.borrow_mut() = *self.matched.borrow();
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
    }
    fn to_string(&self) -> String {
        "ActivationPatternArmFinalize".into()
    }
}
pub(super) struct MatchGate {
    pub(super) matched: Ref<bool>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for MatchGate {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.matched.borrow() {
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardMatchGate".into()
    }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
  }
}
pub(super) struct UnmatchedFinalize {
    pub(super) matched: Ref<bool>,
    pub(super) eligible: Ref<bool>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for UnmatchedFinalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.matched.borrow() {
            Ok(ReactiveSolveStatus::Unchanged)
        } else {
            *self.eligible.borrow_mut() = false;
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        }
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardUnmatchedFinalize".into()
    }
}
pub(super) struct Select {
    pub(super) eligible: Vec<Ref<bool>>,
    pub(super) selected: Ref<usize>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for Select {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.selected.borrow_mut() = self
            .eligible
            .iter()
            .position(|x| *x.borrow())
            .unwrap_or(usize::MAX);
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(Value::Index(self.selected.clone()));
        Ok(values)
    }
    fn to_string(&self) -> String {
        "ActivationPatternSelectArm".into()
    }
}
#[cfg(feature = "compiler")]
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
#[cfg(feature = "compiler")]
interpreter_only!(ScopePulse);
#[cfg(feature = "compiler")]
interpreter_only!(Matcher);
#[cfg(feature = "compiler")]
interpreter_only!(Finalize);
#[cfg(feature = "compiler")]
interpreter_only!(MatchGate);
#[cfg(feature = "compiler")]
interpreter_only!(UnmatchedFinalize);
#[cfg(feature = "compiler")]
interpreter_only!(GuardFinalize);
#[cfg(feature = "compiler")]
interpreter_only!(Select);
