//! A rope-based vector where each element has a height. The intended
//! use is for the elements to be text layout objects.

use xi_rope::tree::{Cursor, Leaf, Node, NodeInfo, TreeBuilder};
use xi_rope::interval::{Interval, IntervalBounds};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Vector<T: Clone>(Node<VectorInfo<T>>);

// This technically doesn't have to be newtyped, we could impl leaf on
// Vec directly.
#[derive(Clone)]
pub struct VectorLeaf<T> {
    data: Vec<T>,
}

// Have to implement by hand because rust issue #26925
impl<T> Default for VectorLeaf<T> {
    fn default() -> Self {
        VectorLeaf { data: Vec::new() }
    }
}

#[derive(Clone)]
pub struct VectorInfo<T> {
    phantom: PhantomData<T>,
}

impl<T: Clone> NodeInfo for VectorInfo<T> {
    type L = VectorLeaf<T>;

    fn accumulate(&mut self, _other: &Self) {}

    fn compute_info(_: &Self::L) -> Self {
        VectorInfo { phantom: Default::default() }
    }
}

const MIN_LEAF: usize = 16;
const MAX_LEAF: usize = 32;

impl<T: Clone> Leaf for VectorLeaf<T> {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_ok_child(&self) -> bool {
        self.data.len() >= MIN_LEAF
    }

    fn push_maybe_split(&mut self, other: &Self, iv: Interval) -> Option<Self> {
        let (start, end) = iv.start_end();
        self.data.extend_from_slice(&other.data[start..end]);
        if self.len() <= MAX_LEAF {
            None
        } else {
            let splitpoint = self.len() / 2;
            let right_vec = self.data.split_off(splitpoint);
            Some(VectorLeaf { data: right_vec })
        }
    }
}

impl<T: Clone> From<Vec<T>> for Vector<T> {
    fn from(v: Vec<T>) -> Self {
        Vector(Node::from_leaf(VectorLeaf { data: v }))
    }
}

impl<T: Clone> Vector<T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn singleton(item: T) -> Vector<T> {
        vec![item].into()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let cursor = Cursor::new(&self.0, index);
        cursor.get_leaf().and_then(|(leaf, offset)| leaf.data.get(offset))
    }

    // Note: we can do get_mut too, but that requires mutable leaf access.

    pub fn push(&mut self, item: T) {
        // This could be optimized more.
        self.0 = Node::concat(self.0.clone(), Self::singleton(item).0)
    }

    pub fn remove(&mut self, index: usize) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn set(&mut self, index: usize, value: T) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(VectorLeaf { data: vec![value]});
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn insert(&mut self, index: usize, value: T) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(VectorLeaf { data: vec![value]});
        self.push_subseq(&mut b, Interval::new(index, self.len()));
        self.0 = b.build();
    }

    pub fn iter_chunks(&self, range: impl IntervalBounds) -> ChunkIter<T> {
        let Interval { start, end } = range.into_interval(self.len());

        ChunkIter { cursor: Cursor::new(&self.0, start), end }
    }

    fn push_subseq(&self, b: &mut TreeBuilder<VectorInfo<T>>, iv: Interval) {
        // TODO: if we make the push_subseq method in xi-rope public, we can save some
        // allocations.
        b.push(self.0.subseq(iv));
    }
}

impl<'a, T: Clone> IntoIterator for &'a Vector<T> {
    type Item = &'a T;

    type IntoIter = std::iter::Flatten<ChunkIter<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_chunks(..).flatten()
    }
}

pub struct ChunkIter<'a, T: Clone> {
    cursor: Cursor<'a, VectorInfo<T>>,
    end: usize,
}

impl<'a, T: Clone> Iterator for ChunkIter<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<&'a [T]> {
        if self.cursor.pos() >= self.end {
            return None;
        }
        let (leaf, start_pos) = self.cursor.get_leaf().unwrap();
        let len = (self.end - self.cursor.pos()).min(leaf.len() - start_pos);
        self.cursor.next_leaf();
        Some(&leaf.data[start_pos..start_pos + len])
    }
}
