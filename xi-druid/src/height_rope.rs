//! A rope-based vector where each element has a height. The intended
//! use is for the elements to be text layout objects.

use xi_rope::tree::{Cursor, Leaf, Node, NodeInfo, TreeBuilder};
use xi_rope::interval::{Interval, IntervalBounds};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Vector<T: Clone>(Node<VectorInfo<T>>);

/// A type representing a height measure.
///
/// Internally this is stored as `usize` using fixed point arithmetic,
/// for two reasons. First, it lets the rope reuse the `Metric` mechanism.
/// Second, it means that the monoid property is exact, which would not be
/// the case for `f64`.
///
/// Currently, there are 8 bits of fraction. On 32 bit platforms, that
/// means a maximum height of 16M, which should be good enough for most
/// practical use but could be a limitation. Of course, on 64 bit platforms,
/// the limit of 7.2e16 should never be a problem.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Height(usize);

impl std::ops::Add for Height {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Height(self.0 + other.0)
    }
}

impl std::ops::AddAssign for Height {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0
    }
}


impl Height {
    /// The number of fractional bits in the representation.
    pub const HEIGHT_FRAC_BITS: usize = 8;

    /// The scale factor for converting from `f64`.
    pub const SCALE_FACTOR: f64 = (1 << Self::HEIGHT_FRAC_BITS) as f64;

    pub fn from_raw_frac(frac: usize) -> Height {
        Height(frac)
    }

    pub fn as_raw_frac(self) -> usize {
        self.0
    }

    pub fn from_f64(height: f64) -> Height {
        Height((height * Self::SCALE_FACTOR).round() as usize)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 * Self::SCALE_FACTOR.recip()
    }
}

// This technically doesn't have to be newtyped, we could impl leaf on
// Vec directly.
#[derive(Clone)]
pub struct VectorLeaf<T> {
    data: Vec<(Height, T)>,
}

// Have to implement by hand because rust issue #26925
impl<T> Default for VectorLeaf<T> {
    fn default() -> Self {
        VectorLeaf { data: Vec::new() }
    }
}

#[derive(Clone)]
pub struct VectorInfo<T> {
    /// The height of this section of rope.
    height: Height,
    phantom: PhantomData<T>,
}

impl<T: Clone> NodeInfo for VectorInfo<T> {
    type L = VectorLeaf<T>;

    fn accumulate(&mut self, other: &Self) {
        self.height += other.height;
    }

    fn compute_info(leaf: &Self::L) -> Self {
        let mut height = Height::default();
        for (leaf_height, _) in &leaf.data {
            height += *leaf_height;
        }
        VectorInfo { height, phantom: Default::default() }
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

impl<T: Clone> From<Vec<(Height, T)>> for Vector<T> {
    fn from(v: Vec<(Height, T)>) -> Self {
        Vector(Node::from_leaf(VectorLeaf { data: v }))
    }
}

// This probably shouldn't expose the internal representation as a pair. A deeper
// question is whether it should even be generic.

impl<T: Clone> Vector<T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn singleton(height: Height, item: T) -> Vector<T> {
        vec![(height, item)].into()
    }

    pub fn get(&self, index: usize) -> Option<&(Height, T)> {
        let cursor = Cursor::new(&self.0, index);
        cursor.get_leaf().and_then(|(leaf, offset)| leaf.data.get(offset))
    }

    pub fn push(&mut self, height: Height, item: T) {
        let el = Self::singleton(height, item);
        // This could be optimized more.
        self.0 = Node::concat(self.0.clone(), el.0)
    }

    // These mutation methods are not super-satisfying; for the general incremental
    // algorithm case, we're going to want to expose builder methods.

    pub fn remove(&mut self, index: usize) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn set(&mut self, index: usize, height: Height, value: T) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(VectorLeaf { data: vec![(height, value)]});
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn insert(&mut self, index: usize, height: Height, value: T) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(VectorLeaf { data: vec![(height, value)]});
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
    // Maybe `(Height, &'a T)` would be better, not to expose the internal
    // tuple, but it's a bit more work.
    type Item = &'a (Height, T);

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
    type Item = &'a [(Height, T)];

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.pos() >= self.end {
            return None;
        }
        let (leaf, start_pos) = self.cursor.get_leaf().unwrap();
        let len = (self.end - self.cursor.pos()).min(leaf.len() - start_pos);
        self.cursor.next_leaf();
        Some(&leaf.data[start_pos..start_pos + len])
    }
}
