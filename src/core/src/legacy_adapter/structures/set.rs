use crate::*;
use indexmap::set::IndexSet;

// Set --------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechSet {
    pub kind: ValueKind,
    /// The exact optional cardinality limit carried by the set's schema.
    ///
    /// `num_elements` predates optional set limits and uses zero as the
    /// unbounded sentinel. Keep it for compatibility, but do not use it when
    /// exact schema identity matters because it cannot distinguish `None`
    /// from `Some(0)`.
    pub max_elements: Option<usize>,
    pub num_elements: usize,
    pub set: IndexSet<LegacyValue>,
}

impl MechSet {
    pub fn new(kind: ValueKind, size: usize) -> MechSet {
        MechSet {
            kind,
            max_elements: (size > 0).then_some(size),
            num_elements: size,
            set: IndexSet::with_capacity(size),
        }
    }

    #[cfg(feature = "pretty_print")]
    pub fn to_html(&self) -> String {
        let mut src = String::new();
        for (i, element) in self.set.iter().enumerate() {
            let e = element.to_html();
            if i == 0 {
                src = format!("{}", e);
            } else {
                src = format!("{}, {}", src, e);
            }
        }
        format!(
            "<span class=\"mech-set\"><span class=\"mech-start-brace\">{{</span>{}<span class=\"mech-end-brace\">}}</span></span>",
            src
        )
    }

    pub fn kind(&self) -> ValueKind {
        ValueKind::Set(Box::new(self.kind.clone()), self.max_elements)
    }

    /// Refreshes the legacy count and exact inferred bound after a set
    /// operation replaces or mutates the contents.
    pub fn sync_cardinality_from_contents(&mut self) {
        self.num_elements = self.set.len();
        self.max_elements = (!self.set.is_empty()).then_some(self.set.len());
    }

    pub fn size_of(&self) -> usize {
        self.set.iter().map(|x| x.size_of()).sum()
    }

    pub fn from_vec(vec: Vec<LegacyValue>) -> MechSet {
        let mut set = IndexSet::new();
        for v in vec {
            set.insert(v);
        }
        let kind = if set.len() > 0 {
            set.iter().next().unwrap().kind()
        } else {
            ValueKind::Empty
        };
        MechSet {
            kind,
            max_elements: (!set.is_empty()).then_some(set.len()),
            num_elements: set.len(),
            set,
        }
    }

    pub fn from_set(set: IndexSet<LegacyValue>) -> MechSet {
        let kind = if set.len() > 0 {
            set.iter().next().unwrap().kind()
        } else {
            ValueKind::Empty
        };
        MechSet {
            kind,
            max_elements: (!set.is_empty()).then_some(set.len()),
            num_elements: set.len(),
            set,
        }
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for MechSet {
    fn pretty_print(&self) -> String {
        fn indent_multiline(value: &str, spaces: usize) -> String {
            let pad = " ".repeat(spaces);
            value
                .lines()
                .map(|line| format!("{pad}{line}"))
                .collect::<Vec<_>>()
                .join("\n")
        }

        let mut lines = Vec::new();
        for element in self.set.iter() {
            lines.push(indent_multiline(&element.pretty_print(), 2));
        }

        if lines.is_empty() {
            "{}".to_string()
        } else {
            format!("{{\n{}\n}}", lines.join(",\n"))
        }
    }
}

impl Hash for MechSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for x in self.set.iter() {
            x.hash(state)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SetKindMismatchError {
    pub expected_kind: ValueKind,
    pub actual_kind: ValueKind,
}
impl MechErrorKind for SetKindMismatchError {
    fn name(&self) -> &str {
        "SetKindMismatch"
    }
    fn message(&self) -> String {
        format!(
            "Schema mismatch: set kind mismatch (expected: {}, found: {}).",
            self.expected_kind, self.actual_kind
        )
    }
}
