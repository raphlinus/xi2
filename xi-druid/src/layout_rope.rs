//! A rope-based vector of layouts.

use std::ops::Range;
use std::sync::Arc;

use druid::piet::{PietTextLayout, TextLayout};

use xi_rope::interval::{Interval, IntervalBounds};
use xi_rope::tree::{Cursor, DefaultMetric, Leaf, Metric, Node, NodeInfo, TreeBuilder};

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

/// An individual layout within the rope.
///
/// Right now, this is just a Piet TextLayout, but we might add more stuff.
pub struct Layout(PietTextLayout);

#[derive(Clone, Default)]
pub struct LayoutRope(Node<LayoutInfo>);

pub struct LayoutRopeBuilder(TreeBuilder<LayoutInfo>);

/// The height metric of the rope, which is in raw Height fractions.
struct HeightMetric;

/// The base metric of the rope, which just counts the number of layouts.
pub struct BaseMetric;

// This technically doesn't have to be newtyped, we could impl leaf on
// the Vec directly, but this feels cleaner.
#[derive(Clone, Default)]
struct LayoutLeaf {
    data: Vec<(Height, Arc<Layout>)>,
}

#[derive(Clone)]
struct LayoutInfo {
    /// The height of this section of rope.
    height: Height,
}

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

    pub const ZERO: Height = Height(0);

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
        self.0 as f64 / Self::SCALE_FACTOR
    }
}

impl Layout {
    pub fn new(inner: PietTextLayout) -> Layout {
        Layout(inner)
    }

    pub fn piet_layout(&self) -> &PietTextLayout {
        &self.0
    }

    pub fn height(&self) -> Height {
        let size = self.0.size();
        Height::from_f64(size.height)
    }
}

impl NodeInfo for LayoutInfo {
    type L = LayoutLeaf;

    fn accumulate(&mut self, other: &Self) {
        self.height += other.height;
    }

    fn compute_info(leaf: &Self::L) -> Self {
        let mut height = Height::ZERO;
        for (leaf_height, _) in &leaf.data {
            height += *leaf_height;
        }
        LayoutInfo { height }
    }
}

impl DefaultMetric for LayoutInfo {
    type DefaultMetric = BaseMetric;
}

const MIN_LEAF: usize = 16;
const MAX_LEAF: usize = 32;

impl Leaf for LayoutLeaf {
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
            Some(LayoutLeaf { data: right_vec })
        }
    }
}

impl From<Vec<(Height, Arc<Layout>)>> for LayoutRope {
    fn from(v: Vec<(Height, Arc<Layout>)>) -> Self {
        LayoutRope(Node::from_leaf(LayoutLeaf { data: v }))
    }
}

impl LayoutRope {
    /// The number of layouts in the rope.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The total height of the rope.
    pub fn height(&self) -> Height {
        Height::from_raw_frac(self.0.measure::<HeightMetric>())
    }

    /// A rope consisting of a single layout.
    pub fn singleton(item: Layout) -> LayoutRope {
        LayoutRope(Node::from_leaf(Self::singleton_leaf(item)))
    }

    fn singleton_leaf(item: Layout) -> LayoutLeaf {
        let height = item.height();
        LayoutLeaf {
            data: vec![(height, Arc::new(item))],
        }
    }

    pub fn get(&self, index: usize) -> Option<(Height, &Layout)> {
        let cursor = Cursor::new(&self.0, index);
        cursor
            .get_leaf()
            .and_then(|(leaf, offset)| leaf.data.get(offset))
            .map(|(height, layout)| (*height, &**layout))
    }

    // These mutation methods might go away in favor of using the builder.

    pub fn push(&mut self, item: Layout) {
        let el = Self::singleton(item);
        // This could be optimized more.
        self.0 = Node::concat(self.0.clone(), el.0)
    }

    pub fn remove(&mut self, index: usize) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn set(&mut self, index: usize, item: Layout) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(Self::singleton_leaf(item));
        self.push_subseq(&mut b, Interval::new(index + 1, self.len()));
        self.0 = b.build();
    }

    pub fn insert(&mut self, index: usize, value: Layout) {
        let mut b = TreeBuilder::new();
        self.push_subseq(&mut b, Interval::new(0, index));
        b.push_leaf(Self::singleton_leaf(value));
        self.push_subseq(&mut b, Interval::new(index, self.len()));
        self.0 = b.build();
    }

    fn iter_chunks(&self, range: impl IntervalBounds) -> ChunkIter {
        let Interval { start, end } = range.into_interval(self.len());

        ChunkIter {
            cursor: Cursor::new(&self.0, start),
            end,
        }
    }

    /// The height at the top of the layout at the given index.
    ///
    /// This is simply the sum of the heights of the layouts that come before
    /// it.
    pub fn height_of_index(&self, index: usize) -> Height {
        Height::from_raw_frac(self.0.count::<HeightMetric>(index))
    }

    /// The layout at the given height.
    ///
    /// Edge cases get interesting (especially since zero-height layouts are
    /// not forbidden), so here is a more precise spec: it is the first layout
    /// that either contains (in the closed-open interval sense) the given
    /// height, or is a zero-height layout at the given height.
    ///
    /// If the total height is given and the rope does not end on a zero-height
    /// layout, then it returns the number of layouts.
    ///
    /// TODO: is there a simpler way to state that? It seems more complicated
    /// than it should be.
    pub fn index_of_height(&self, height: Height) -> usize {
        self.0
            .count_base_units::<HeightMetric>(height.as_raw_frac())
    }

    fn push_subseq(&self, b: &mut TreeBuilder<LayoutInfo>, iv: Interval) {
        // TODO: if we make the push_subseq method in xi-rope public, we can save some
        // allocations.
        b.push(self.0.subseq(iv));
    }
}

impl LayoutRopeBuilder {
    pub fn new() -> LayoutRopeBuilder {
        LayoutRopeBuilder(TreeBuilder::new())
    }

    #[allow(unused)]
    pub fn push_rope_slice(&mut self, other: &LayoutRope, range: Range<usize>) {
        // TODO: use push_subseq method on TreeBuilder when that lands.
        self.0.push(other.0.subseq(Interval::from(range)))
    }

    pub fn push_layout(&mut self, layout: Layout) {
        // Maybe move the body of singleton_leaf to here?
        self.0.push_leaf(LayoutRope::singleton_leaf(layout))
    }

    pub fn build(self) -> LayoutRope {
        LayoutRope(self.0.build())
    }
}

impl<'a> IntoIterator for &'a LayoutRope {
    // Maybe `(Height, &'a Layout)` would be better, not to expose the internal
    // representation, but it's a bit more work.
    type Item = &'a (Height, Arc<Layout>);

    type IntoIter = std::iter::Flatten<ChunkIter<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_chunks(..).flatten()
    }
}

pub struct ChunkIter<'a> {
    cursor: Cursor<'a, LayoutInfo>,
    end: usize,
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = &'a [(Height, Arc<Layout>)];

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

impl Metric<LayoutInfo> for BaseMetric {
    fn measure(_: &LayoutInfo, len: usize) -> usize {
        len
    }

    fn to_base_units(_l: &LayoutLeaf, in_measured_units: usize) -> usize {
        in_measured_units
    }

    fn from_base_units(_l: &LayoutLeaf, in_base_units: usize) -> usize {
        in_base_units
    }

    fn is_boundary(_l: &LayoutLeaf, _offset: usize) -> bool {
        true
    }

    fn prev(_l: &LayoutLeaf, offset: usize) -> Option<usize> {
        Some(offset - 1)
    }

    fn next(_l: &LayoutLeaf, offset: usize) -> Option<usize> {
        Some(offset + 1)
    }

    fn can_fragment() -> bool {
        false
    }
}

impl Metric<LayoutInfo> for HeightMetric {
    fn measure(info: &LayoutInfo, _len: usize) -> usize {
        info.height.as_raw_frac()
    }

    fn from_base_units(l: &LayoutLeaf, in_base_units: usize) -> usize {
        let mut height = Height::ZERO;
        for (h, _el) in &l.data[..in_base_units] {
            height += *h;
        }
        height.as_raw_frac()
    }

    fn to_base_units(l: &LayoutLeaf, in_measured_units: usize) -> usize {
        let mut m1 = in_measured_units;
        let mut m2 = 0;
        for (h, _el) in &l.data {
            if m1 == 0 || m1 < h.as_raw_frac() {
                break;
            }
            m1 -= h.as_raw_frac();
            m2 += 1;
        }
        m2
    }

    fn is_boundary(_l: &LayoutLeaf, _offset: usize) -> bool {
        true
    }

    fn prev(_l: &LayoutLeaf, offset: usize) -> Option<usize> {
        Some(offset - 1)
    }

    fn next(_l: &LayoutLeaf, offset: usize) -> Option<usize> {
        Some(offset + 1)
    }

    fn can_fragment() -> bool {
        // The documentation in xi-rope is confusing (TODO: fix that),
        // but basically this predicate asks whether a nonempty leaf
        // may contain zero measure. Since we're not disallowing that,
        // we say "yes" here. If we did disallow zero-height layouts,
        // then this stuff would be (slightly) more efficient.
        true
    }
}
