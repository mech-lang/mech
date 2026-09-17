//! Persistent, ordered storage. The left child is always a full power-of-two
//! subtree, so append path copying and lookup are logarithmic. No syntax nodes
//! or grammar boundaries are introduced by this storage representation.
use alloc::sync::Arc;

pub(crate) trait Measured {
    fn measure(&self) -> usize;
}

#[derive(Debug, Eq, PartialEq)]
enum Node<T> {
    Leaf(T),
    Branch {
        left: Arc<Node<T>>,
        right: Arc<Node<T>>,
        len: usize,
        measure: usize,
    },
}

impl<T: Measured> Node<T> {
    fn len(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Branch { len, .. } => *len,
        }
    }
    fn measure(&self) -> usize {
        match self {
            Self::Leaf(value) => value.measure(),
            Self::Branch { measure, .. } => *measure,
        }
    }
    fn branch(left: Arc<Self>, right: Arc<Self>, allocations: &mut u64) -> Arc<Self> {
        *allocations += 1;
        Arc::new(Self::Branch {
            len: left.len() + right.len(),
            measure: left.measure() + right.measure(),
            left,
            right,
        })
    }
    fn append(old: &Arc<Self>, leaf: Arc<Self>, allocations: &mut u64) -> Arc<Self> {
        if old.len().is_power_of_two() {
            return Self::branch(old.clone(), leaf, allocations);
        }
        let Self::Branch { left, right, .. } = old.as_ref() else {
            unreachable!()
        };
        Self::branch(
            left.clone(),
            Self::append(right, leaf, allocations),
            allocations,
        )
    }
    fn replace_last(old: &Arc<Self>, leaf: Arc<Self>, allocations: &mut u64) -> Arc<Self> {
        match old.as_ref() {
            Self::Leaf(_) => leaf,
            Self::Branch { left, right, .. } => Self::branch(
                left.clone(),
                Self::replace_last(right, leaf, allocations),
                allocations,
            ),
        }
    }
    fn get(&self, index: usize) -> Option<&T> {
        match self {
            Self::Leaf(value) => (index == 0).then_some(value),
            Self::Branch { left, right, .. } => {
                if index < left.len() {
                    left.get(index)
                } else {
                    right.get(index - left.len())
                }
            }
        }
    }
    fn at_measure(&self, offset: usize, base: usize) -> Option<(usize, &T)> {
        match self {
            Self::Leaf(value) => (offset < value.measure()).then_some((base, value)),
            Self::Branch { left, right, .. } => {
                let width = left.measure();
                if offset < width {
                    left.at_measure(offset, base)
                } else {
                    right.at_measure(offset - width, base + width)
                }
            }
        }
    }
    fn visit(&self, start: usize, end: usize, base: usize, f: &mut impl FnMut(usize, &T)) {
        if start >= base + self.measure() || end <= base {
            return;
        }
        match self {
            Self::Leaf(value) => f(base, value),
            Self::Branch { left, right, .. } => {
                left.visit(start, end, base, f);
                right.visit(start, end, base + left.measure(), f);
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RetainedSequence<T> {
    root: Option<Arc<Node<T>>>,
}

impl<T> Clone for RetainedSequence<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}
impl<T> Default for RetainedSequence<T> {
    fn default() -> Self {
        Self { root: None }
    }
}
impl<T: Measured> RetainedSequence<T> {
    pub(crate) fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |node| node.len())
    }
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.root.as_ref()?.get(index)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        let mut iter = SequenceIter {
            stack: [None; usize::BITS as usize + 1],
            len: 0,
        };
        if let Some(root) = &self.root {
            iter.push(root);
        }
        iter
    }
    pub(crate) fn appended(&self, value: T) -> (Self, u64) {
        let leaf = Arc::new(Node::Leaf(value));
        let mut allocations = 1;
        let root = match &self.root {
            None => leaf,
            Some(root) => Node::append(root, leaf, &mut allocations),
        };
        (Self { root: Some(root) }, allocations)
    }
    pub(crate) fn replacing_last(&self, value: T) -> (Self, u64) {
        let leaf = Arc::new(Node::Leaf(value));
        let mut allocations = 1;
        let root = match &self.root {
            None => leaf,
            Some(root) => Node::replace_last(root, leaf, &mut allocations),
        };
        (Self { root: Some(root) }, allocations)
    }
    pub(crate) fn at_measure(&self, offset: usize) -> Option<(usize, &T)> {
        self.root.as_ref()?.at_measure(offset, 0)
    }
    pub(crate) fn visit_range(&self, start: usize, end: usize, mut f: impl FnMut(usize, &T)) {
        if let Some(root) = &self.root {
            root.visit(start, end, 0, &mut f);
        }
    }
    pub(crate) fn node_bytes() -> usize {
        core::mem::size_of::<Node<T>>()
    }
}
impl<T: Measured> FromIterator<T> for RetainedSequence<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().fold(Self::default(), |sequence, value| {
            sequence.appended(value).0
        })
    }
}
impl<T: Measured> core::ops::Index<usize> for RetainedSequence<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.get(index).expect("retained index")
    }
}

// A sequence cannot have more elements than usize can address; the complete
// left-subtree invariant bounds the traversal stack independently of input.
struct SequenceIter<'a, T> {
    stack: [Option<&'a Node<T>>; usize::BITS as usize + 1],
    len: usize,
}
impl<'a, T> SequenceIter<'a, T> {
    fn push(&mut self, node: &'a Node<T>) {
        self.stack[self.len] = Some(node);
        self.len += 1;
    }
}
impl<'a, T> Iterator for SequenceIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        while self.len > 0 {
            self.len -= 1;
            match self.stack[self.len]
                .take()
                .expect("retained traversal frame")
            {
                Node::Leaf(value) => return Some(value),
                Node::Branch { left, right, .. } => {
                    self.push(right);
                    self.push(left);
                }
            }
        }
        None
    }
}
