//! Conservative straight-line comparison reuse after register allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Compare {
    pub wide: bool,
    pub left: u8,
    pub right: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Compare(Compare),
    Select(u8),
    /// Writes one integer register while preserving NZCV.
    Write(u8),
    Barrier,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tracker {
    comparison: Option<Compare>,
    selected: bool,
}
pub(crate) struct Plan {
    pub elide: bool,
    pub next: Tracker,
}
impl Tracker {
    pub fn plan(self, action: Action) -> Plan {
        match action {
            Action::Compare(comparison) => Plan {
                elide: self.selected && self.comparison == Some(comparison),
                next: Self {
                    comparison: Some(comparison),
                    selected: false,
                },
            },
            Action::Select(destination) | Action::Write(destination) => {
                let retains = self
                    .comparison
                    .is_some_and(|c| destination != c.left && destination != c.right);
                Plan {
                    elide: false,
                    next: if retains {
                        Self {
                            comparison: self.comparison,
                            selected: self.selected || matches!(action, Action::Select(_)),
                        }
                    } else {
                        Self::default()
                    },
                }
            }
            Action::Barrier => Plan {
                elide: false,
                next: Self::default(),
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const C: Compare = Compare {
        wide: false,
        left: 0,
        right: 1,
    };
    fn prefix() -> Tracker {
        Tracker::default()
            .plan(Action::Compare(C))
            .next
            .plan(Action::Select(8))
            .next
    }
    #[test]
    fn exact_selected_compare_is_redundant() {
        assert!(prefix().plan(Action::Compare(C)).elide);
    }
    #[test]
    fn comparison_without_a_selection_is_not_elided() {
        assert!(
            !Tracker::default()
                .plan(Action::Compare(C))
                .next
                .plan(Action::Compare(C))
                .elide
        );
    }
    #[test]
    fn unrelated_writes_retain_current_flags() {
        let t = prefix().plan(Action::Write(8)).next;
        assert!(t.plan(Action::Compare(C)).elide);
    }
    #[test]
    fn every_operand_alias_invalidates() {
        for r in [0, 1] {
            for a in [Action::Write(r), Action::Select(r)] {
                assert!(!prefix().plan(a).next.plan(Action::Compare(C)).elide);
            }
        }
    }
    #[test]
    fn different_width_order_or_register_declines() {
        for c in [
            Compare { wide: true, ..C },
            Compare {
                left: 1,
                right: 0,
                ..C
            },
            Compare { left: 2, ..C },
            Compare { right: 2, ..C },
        ] {
            assert!(!prefix().plan(Action::Compare(c)).elide);
        }
    }
    #[test]
    fn every_unknown_effect_is_a_barrier() {
        assert!(
            !prefix()
                .plan(Action::Barrier)
                .next
                .plan(Action::Compare(C))
                .elide
        );
    }
    #[test]
    fn fresh_blocks_have_no_inherited_flags() {
        assert!(!Tracker::default().plan(Action::Compare(C)).elide);
    }
    #[test]
    fn elision_requires_a_new_selection_before_another_elision() {
        let p = prefix().plan(Action::Compare(C));
        assert!(p.elide);
        assert!(!p.next.plan(Action::Compare(C)).elide);
        assert!(
            p.next
                .plan(Action::Select(9))
                .next
                .plan(Action::Compare(C))
                .elide
        );
    }
}
