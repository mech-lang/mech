//! Persistent, ordered storage. The left child is always a full power-of-two
//! subtree, so append path copying and lookup are logarithmic. No syntax nodes
//! or grammar boundaries are introduced by this storage representation.
use alloc::sync::Arc;

pub(crate) trait Measured {
    fn measure(&self) -> usize;
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    fn truncate(old: &Arc<Self>, len: usize, allocations: &mut u64) -> Arc<Self> {
        if len >= old.len() {
            return old.clone();
        }
        let Self::Branch { left, right, .. } = old.as_ref() else {
            unreachable!("nonempty retained prefix")
        };
        if len <= left.len() {
            Self::truncate(left, len, allocations)
        } else {
            Self::branch(
                left.clone(),
                Self::truncate(right, len - left.len(), allocations),
                allocations,
            )
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
    fn at_measure(&self, offset: usize, base: usize, steps: &mut u64) -> Option<(usize, &T)> {
        *steps += 1;
        match self {
            Self::Leaf(value) => (offset < value.measure()).then_some((base, value)),
            Self::Branch { left, right, .. } => {
                let width = left.measure();
                if offset < width {
                    left.at_measure(offset, base, steps)
                } else {
                    right.at_measure(offset - width, base + width, steps)
                }
            }
        }
    }
    fn visit(
        &self,
        start: usize,
        end: usize,
        base: usize,
        f: &mut impl FnMut(usize, &T),
        steps: &mut u64,
    ) {
        *steps += 1;
        if start >= base + self.measure() || end <= base {
            return;
        }
        match self {
            Self::Leaf(value) => f(base, value),
            Self::Branch { left, right, .. } => {
                left.visit(start, end, base, f, steps);
                right.visit(start, end, base + left.measure(), f, steps);
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
    pub(crate) fn iter_with_work(&self) -> impl Iterator<Item = (&T, u64)> {
        let mut iter = SequenceIter {
            stack: [None; usize::BITS as usize + 1],
            len: 0,
        };
        if let Some(root) = &self.root {
            iter.push(root);
        }
        core::iter::from_fn(move || iter.next_counted())
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
    pub(crate) fn at_measure_with_work(&self, offset: usize) -> (Option<(usize, &T)>, u64) {
        let mut steps = 0;
        let value = self
            .root
            .as_ref()
            .and_then(|root| root.at_measure(offset, 0, &mut steps));
        (value, steps)
    }
    pub(crate) fn visit_range_with_work(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(usize, &T),
    ) -> u64 {
        let mut steps = 0;
        if let Some(root) = &self.root {
            root.visit(start, end, 0, &mut f, &mut steps);
        }
        steps
    }
    pub(crate) fn truncated(&self, len: usize) -> (Self, u64) {
        if len == 0 {
            return (Self::default(), 0);
        }
        let mut allocations = 0;
        let root = self
            .root
            .as_ref()
            .map(|root| Node::truncate(root, len, &mut allocations));
        (Self { root }, allocations)
    }
    pub(crate) fn into_values(self) -> impl Iterator<Item = T>
    where
        T: Clone,
    {
        struct Owned<T> {
            stack: alloc::vec::Vec<Arc<Node<T>>>,
        }
        impl<T: Clone> Iterator for Owned<T> {
            type Item = T;
            fn next(&mut self) -> Option<T> {
                while let Some(node) = self.stack.pop() {
                    match node.as_ref() {
                        Node::Leaf(value) => return Some(value.clone()),
                        Node::Branch { left, right, .. } => {
                            self.stack.push(right.clone());
                            self.stack.push(left.clone());
                        }
                    }
                }
                None
            }
        }
        Owned {
            stack: self.root.into_iter().collect(),
        }
    }
    pub(crate) fn iter_from(&self, mut index: usize) -> impl Iterator<Item = &T> {
        let mut iter = SequenceIter {
            stack: [None; usize::BITS as usize + 1],
            len: 0,
        };
        if index < self.len() {
            let mut node = self.root.as_deref().expect("retained root");
            loop {
                match node {
                    Node::Leaf(_) => {
                        iter.push(node);
                        break;
                    }
                    Node::Branch { left, right, .. } => {
                        if index < left.len() {
                            iter.push(right);
                            node = left;
                        } else {
                            index -= left.len();
                            node = right;
                        }
                    }
                }
            }
        }
        iter
    }
    pub(crate) fn node_bytes() -> usize {
        core::mem::size_of::<Node<T>>()
    }
}
// Mutation is restricted to unit-measured journal entries: path copying
// cannot invalidate cached subtree widths.
pub(crate) trait UnitMeasured: Measured {}
impl<T: UnitMeasured + Clone> RetainedSequence<T> {
    pub(crate) fn get_mut(&mut self, index: usize, allocations: &mut u64) -> Option<&mut T> {
        fn descend<'a, T: UnitMeasured + Clone>(
            node: &'a mut Arc<Node<T>>,
            index: usize,
            allocations: &mut u64,
        ) -> Option<&'a mut T> {
            if Arc::strong_count(node) > 1 {
                *allocations += 1;
            }
            match Arc::make_mut(node) {
                Node::Leaf(value) => (index == 0).then_some(value),
                Node::Branch { left, right, .. } => {
                    if index < left.len() {
                        descend(left, index, allocations)
                    } else {
                        let index = index - left.len();
                        descend(right, index, allocations)
                    }
                }
            }
        }
        if index >= self.len() {
            return None;
        }
        descend(self.root.as_mut()?, index, allocations)
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
        self.next_counted().map(|(value, _)| value)
    }
}
impl<'a, T> SequenceIter<'a, T> {
    fn next_counted(&mut self) -> Option<(&'a T, u64)> {
        let mut steps = 0;
        while self.len > 0 {
            steps += 1;
            self.len -= 1;
            match self.stack[self.len]
                .take()
                .expect("retained traversal frame")
            {
                Node::Leaf(value) => return Some((value, steps)),
                Node::Branch { left, right, .. } => {
                    self.push(right);
                    self.push(left);
                }
            }
        }
        None
    }
}
