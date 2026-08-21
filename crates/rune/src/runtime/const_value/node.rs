//! The array a [`ConstValue`] is made of.
//!
//! A constant is an owned tree, and a tree which owns its parts is built,
//! walked, cloned and dropped by recursing over it unless something is done
//! about it. So it is not stored as a tree at all: it is stored as one array of
//! nodes in pre-order, in which every subtree is a contiguous run.
//!
//! That makes dropping and cloning a constant the same operation as dropping
//! and cloning one array, and makes reading one back off disk a linear pass
//! which *checks* how deeply the array nests rather than a descent which finds
//! out by surviving.

use core::fmt;
use core::mem::take;
use core::ops::{Deref, DerefMut};

#[cfg(feature = "musli")]
use musli_core::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::runtime::{AnyTypeInfo, Bytes, Inline, Object, OwnedTuple, TypeInfo, Value};
use crate::{self as rune};
use crate::{Hash, TypeHash};

use super::{MAX_CONST_DEPTH, MAX_CONST_SIZE};

/// One node of the array a [`ConstValue`] is made of.
///
/// The nodes of a subtree are laid out in pre-order, so `size` is what says
/// where one subtree ends and the next begins - it is how many nodes this node
/// and everything below it occupy, so a leaf is `1`.
///
/// It is derived rather than stored: whoever hands over an array hands over the
/// kinds, and the sizes are worked out from them, so there is nothing for a
/// size to disagree with.
#[derive(Debug, TryClone)]
pub(crate) struct ConstNode {
    #[try_clone(copy)]
    pub(crate) size: u32,
    /// How deeply this node's subtree nests, itself included. A leaf is `1`.
    ///
    /// Keeping it means the depth of a constant is known without walking it,
    /// which is what lets the bound be checked wherever one is built out of
    /// others rather than only where one is built from scratch.
    #[try_clone(copy)]
    pub(crate) height: u32,
    pub(crate) kind: ConstNodeKind,
}

/// What one node of a [`ConstValue`] is.
///
/// Only the counts are stored - which nodes are a node's children is decided by
/// where they are, which is what makes the array a tree.
#[derive(Debug, TryClone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
pub(crate) enum ConstNodeKind {
    /// An inline constant value.
    Inline(#[try_clone(copy)] Inline),
    /// A string constant.
    String(Box<str>),
    /// A byte string.
    Bytes(Box<[u8]>),
    /// An instance of some type of value, made of the `fields` subtrees which
    /// follow it.
    Instance {
        /// The type hash of the value. If the value is a variant, this is the
        /// type hash of the enum.
        #[try_clone(copy)]
        hash: Hash,
        /// The type hash of the variant, or [`Hash::EMPTY`] if this is not an
        /// enum.
        #[try_clone(copy)]
        variant_hash: Hash,
        /// How many subtrees follow.
        #[try_clone(copy)]
        fields: u32,
    },
    /// An object, where `keys[n]` names the `n`th of the `keys.len()` subtrees
    /// which follow it.
    ///
    /// The keys are kept sorted, so that whoever reads one back does not have
    /// to sort them again.
    Object { keys: Box<[Box<str>]> },
}

impl ConstNodeKind {
    /// How many subtrees follow this node.
    #[inline]
    pub(crate) fn children(&self) -> usize {
        match self {
            ConstNodeKind::Instance { fields, .. } => *fields as usize,
            ConstNodeKind::Object { keys } => keys.len(),
            _ => 0,
        }
    }

    pub(crate) fn type_info(&self) -> TypeInfo {
        fn struct_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unknown constant struct")
        }

        fn variant_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unknown constant variant")
        }

        match self {
            ConstNodeKind::Inline(value) => value.type_info(),
            ConstNodeKind::String(..) => TypeInfo::any::<crate::alloc::String>(),
            ConstNodeKind::Bytes(..) => TypeInfo::any::<Bytes>(),
            ConstNodeKind::Object { .. } => TypeInfo::any::<Object>(),
            ConstNodeKind::Instance {
                hash, variant_hash, ..
            } => match *hash {
                Option::<Value>::HASH => TypeInfo::any::<Option<Value>>(),
                crate::runtime::Vec::HASH => TypeInfo::any::<crate::runtime::Vec>(),
                OwnedTuple::HASH => TypeInfo::any::<OwnedTuple>(),
                Object::HASH => TypeInfo::any::<Object>(),
                hash if *variant_hash == Hash::EMPTY => {
                    TypeInfo::any_type_info(AnyTypeInfo::new(struct_name, hash))
                }
                hash => TypeInfo::any_type_info(AnyTypeInfo::new(variant_name, hash)),
            },
        }
    }
}

/// What is wrong with an array which does not describe a constant.
///
/// An array is only ever built here, so this is what a *decoded* one can be:
/// whoever wrote it decided what is in it, and a `.rnc` file is read back from
/// disk.
#[derive(Debug)]
pub(crate) enum ConstNodesError {
    /// The array is empty, and every constant has at least a root.
    #[cfg(any(feature = "serde", feature = "musli"))]
    Empty,
    /// A node claims more subtrees follow it than the array has left, or the
    /// array has nodes left over once the root's subtree ends.
    #[cfg(any(feature = "serde", feature = "musli"))]
    Malformed,
    /// The array nests deeper than a constant is allowed to.
    TooDeep { max: usize },
    /// The array is made of more nodes than a constant is allowed to be.
    TooLarge { max: usize },
    /// Memory ran out while the array was being checked.
    Alloc(alloc::Error),
}

impl From<alloc::Error> for ConstNodesError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        ConstNodesError::Alloc(error)
    }
}

impl fmt::Display for ConstNodesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(any(feature = "serde", feature = "musli"))]
            ConstNodesError::Empty => write!(f, "Constant value is empty"),
            #[cfg(any(feature = "serde", feature = "musli"))]
            ConstNodesError::Malformed => {
                write!(f, "Constant value is not a tree")
            }
            ConstNodesError::TooDeep { max } => {
                write!(f, "Constant value is nested too deeply, limit is {max}")
            }
            ConstNodesError::TooLarge { max } => {
                write!(
                    f,
                    "Constant value is made of too many values, limit is {max}"
                )
            }
            ConstNodesError::Alloc(error) => error.fmt(f),
        }
    }
}

impl core::error::Error for ConstNodesError {}

/// Work out the size of every subtree in `kinds`, checking as it goes that the
/// array describes one tree and that the tree is within the limits.
///
/// The walk goes backwards, so that a node is reached once everything below it
/// has been. Each node takes the subtrees of its children off the stack, which
/// is what says the array is a tree: running out of them means a node claimed
/// children it does not have, and having any left at the end means the array
/// holds more than the root's subtree.
#[cfg(any(feature = "serde", feature = "musli"))]
fn measure(kinds: &[ConstNodeKind]) -> Result<Vec<(u32, u32)>, ConstNodesError> {
    let len = kinds.len();

    if len == 0 {
        return Err(ConstNodesError::Empty);
    }

    if len > MAX_CONST_SIZE {
        return Err(ConstNodesError::TooLarge {
            max: MAX_CONST_SIZE,
        });
    }

    let mut sizes = Vec::new();
    let mut heights = Vec::new();
    sizes.try_resize(len, 0u32)?;
    heights.try_resize(len, 0u32)?;

    // The subtrees which are complete but whose parent has not been reached
    // yet, most recent first - so popping hands them back in the order they
    // appear in the array.
    let mut pending = Vec::new();

    for index in (0..len).rev() {
        let mut size = 1u32;
        let mut height = 1u32;

        for _ in 0..kinds[index].children() {
            let child: usize = pending.pop().ok_or(ConstNodesError::Malformed)?;

            size = size
                .checked_add(sizes[child])
                .ok_or(ConstNodesError::Malformed)?;

            height = height.max(heights[child].saturating_add(1));
        }

        sizes[index] = size;
        heights[index] = height;
        pending.try_push(index)?;
    }

    // Everything below the root has been taken by something, and the root is
    // what is left.
    if pending.len() != 1 {
        return Err(ConstNodesError::Malformed);
    }

    if heights[0] as usize > MAX_CONST_DEPTH {
        return Err(ConstNodesError::TooDeep {
            max: MAX_CONST_DEPTH,
        });
    }

    let mut out = Vec::try_with_capacity(len)?;

    for index in 0..len {
        out.try_push((sizes[index], heights[index]))?;
    }

    Ok(out)
}

/// The fields of an instance or the values of an object.
///
/// A field is a subtree rather than an element, so this is a view over the run
/// which holds them all rather than a slice of them.
#[derive(Clone, Copy)]
pub struct ConstFields<'a> {
    nodes: &'a [ConstNode],
    len: usize,
}

impl<'a> ConstFields<'a> {
    /// An empty set of fields.
    pub(crate) const EMPTY: ConstFields<'static> = ConstFields { nodes: &[], len: 0 };

    /// How many fields there are.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no fields.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the field at `index`.
    ///
    /// This walks past the fields before it, since a field is as long as the
    /// subtree it is.
    pub fn get(&self, index: usize) -> Option<&'a ConstValue> {
        if index >= self.len {
            return None;
        }

        let mut nodes = self.nodes;

        for _ in 0..index {
            let size = nodes.first()?.size as usize;
            nodes = nodes.get(size..)?;
        }

        let size = nodes.first()?.size as usize;
        Some(ConstValue::from_nodes(nodes.get(..size)?))
    }

    /// Iterate over the fields in order.
    #[inline]
    pub fn iter(&self) -> ConstFieldsIter<'a> {
        ConstFieldsIter {
            nodes: self.nodes,
            len: self.len,
        }
    }
}

impl fmt::Debug for ConstFields<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a> IntoIterator for ConstFields<'a> {
    type Item = &'a ConstValue;
    type IntoIter = ConstFieldsIter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over the fields of an instance, see [`ConstFields::iter`].
#[derive(Clone)]
pub struct ConstFieldsIter<'a> {
    nodes: &'a [ConstNode],
    len: usize,
}

impl<'a> Iterator for ConstFieldsIter<'a> {
    type Item = &'a ConstValue;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let size = self.nodes.first()?.size as usize;
        let (head, tail) = self.nodes.split_at_checked(size)?;
        self.nodes = tail;
        self.len -= 1;
        Some(ConstValue::from_nodes(head))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl ExactSizeIterator for ConstFieldsIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.len
    }
}

/// A constant value.
///
/// This is the borrowed half of the pair, the way [`Item`] is to [`ItemBuf`] -
/// a subtree of a constant is a run of the array its parent is stored in, so a
/// field of a constant is one of these rather than something which had to be
/// copied out.
///
/// [`Item`]: crate::Item
/// [`ItemBuf`]: crate::ItemBuf
#[repr(transparent)]
pub struct ConstValue {
    nodes: [ConstNode],
}

impl ConstValue {
    /// View a run of nodes as the constant it describes.
    ///
    /// The run has to be one whole subtree, which is what everything handing
    /// one out here is careful to pass.
    #[inline]
    pub(crate) fn from_nodes(nodes: &[ConstNode]) -> &Self {
        // SAFETY: `ConstValue` is `#[repr(transparent)]` over `[ConstNode]`.
        unsafe { &*(nodes as *const [ConstNode] as *const ConstValue) }
    }

    /// The nodes this constant is made of, its root first.
    #[inline]
    pub(crate) fn as_nodes(&self) -> &[ConstNode] {
        &self.nodes
    }

    /// What the root of this constant is.
    #[inline]
    pub(crate) fn kind(&self) -> &ConstNodeKind {
        // A constant is never empty - see `measure` and `ConstBuilder::build`.
        match self.nodes.first() {
            Some(node) => &node.kind,
            None => &ConstNodeKind::Inline(Inline::Empty),
        }
    }

    /// The subtrees which follow the root, whatever it is.
    ///
    /// This is what the code written by the [`ToConstValue`] derive reads a
    /// constant's fields back out of.
    ///
    /// [`ToConstValue`]: derive@crate::ToConstValue
    #[inline]
    pub fn fields(&self) -> ConstFields<'_> {
        let Some((_, rest)) = self.nodes.split_first() else {
            return ConstFields::EMPTY;
        };

        ConstFields {
            nodes: rest,
            len: self.kind().children(),
        }
    }

    /// What the root of this constant is, so that it can be changed in place.
    ///
    /// Only the leaves an inline value is stored in are reachable this way,
    /// which is what keeps the shape of the array from being changed under it.
    #[inline]
    pub(crate) fn kind_mut(&mut self) -> Option<&mut ConstNodeKind> {
        Some(&mut self.nodes.first_mut()?.kind)
    }

    /// How deeply this constant nests, itself included - so a scalar is `1`.
    ///
    /// Every constant which exists is within [`MAX_CONST_DEPTH`], which is
    /// checked wherever one is built and wherever one is read back.
    #[inline]
    pub(crate) fn height(&self) -> u32 {
        match self.nodes.first() {
            Some(node) => node.height,
            None => 0,
        }
    }

    /// Get the type information of the value.
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        self.kind().type_info()
    }
}

impl fmt::Debug for ConstValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self::debug::fmt(self, f)
    }
}

impl TryToOwned for ConstValue {
    type Owned = ConstValueBuf;

    #[inline]
    fn try_to_owned(&self) -> alloc::Result<Self::Owned> {
        ConstValueBuf::from_vec(self.nodes.try_to_owned()?)
    }
}

/// A constant value which owns what it is made of.
///
/// This is the owned half of the pair, the way [`ItemBuf`] is to [`Item`].
/// Everything it is made of is in one allocation, so cloning one is one copy
/// and dropping one is one deallocation, whatever shape the constant is.
///
/// [`Item`]: crate::Item
/// [`ItemBuf`]: crate::ItemBuf
pub struct ConstValueBuf {
    nodes: Nodes,
}

/// Where the nodes of an owned constant are kept.
///
/// Most constants are one scalar, so the one-node case is kept inline and a
/// constant only allocates once it is made of something.
enum Nodes {
    One(ConstNode),
    Many(Box<[ConstNode]>),
}

impl Nodes {
    #[inline]
    fn as_slice(&self) -> &[ConstNode] {
        match self {
            Nodes::One(node) => core::slice::from_ref(node),
            Nodes::Many(nodes) => nodes,
        }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [ConstNode] {
        match self {
            Nodes::One(node) => core::slice::from_mut(node),
            Nodes::Many(nodes) => nodes,
        }
    }
}

impl TryClone for Nodes {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(match self {
            Nodes::One(node) => Nodes::One(node.try_clone()?),
            Nodes::Many(nodes) => Nodes::Many(nodes.try_clone()?),
        })
    }
}

impl ConstValueBuf {
    /// Build one from a run of nodes which is known to be a whole subtree.
    pub(crate) fn from_vec(nodes: Vec<ConstNode>) -> alloc::Result<Self> {
        let mut nodes = nodes;

        // A constant which is one node is kept inline, which is what a scalar
        // constant costs - nothing.
        if nodes.len() == 1 {
            if let Some(node) = nodes.pop() {
                return Ok(Self {
                    nodes: Nodes::One(node),
                });
            }
        }

        Ok(Self {
            nodes: Nodes::Many(nodes.try_into_boxed_slice()?),
        })
    }

    /// Build one from the kinds an array is made of, working out the shape and
    /// checking that it is a tree within the limits.
    ///
    /// This is what reading a constant back from somewhere else goes through,
    /// so it is where an array which is not a constant is turned away.
    #[cfg(any(feature = "serde", feature = "musli"))]
    pub(crate) fn from_kinds(kinds: Vec<ConstNodeKind>) -> Result<Self, ConstNodesError> {
        let sizes = measure(&kinds)?;

        let mut nodes = Vec::try_with_capacity(kinds.len())?;

        for ((size, height), kind) in sizes.into_iter().zip(kinds) {
            nodes.try_push(ConstNode { size, height, kind })?;
        }

        Ok(Self::from_vec(nodes)?)
    }

    /// The kinds this constant is made of, which is what is written down when
    /// one is handed to somebody else.
    #[cfg(any(feature = "serde", feature = "musli"))]
    #[inline]
    pub(crate) fn kinds(&self) -> impl ExactSizeIterator<Item = &ConstNodeKind> + '_ {
        self.nodes.as_slice().iter().map(|node| &node.kind)
    }
}

impl ConstValueBuf {
    /// Build one out of a single node which has nothing below it.
    ///
    /// This is where most constants come from and it does not allocate.
    #[inline]
    pub(crate) fn from_kind(kind: ConstNodeKind) -> Self {
        Self {
            nodes: Nodes::One(ConstNode {
                size: 1,
                height: 1,
                kind,
            }),
        }
    }
}

impl Deref for ConstValueBuf {
    type Target = ConstValue;

    #[inline]
    fn deref(&self) -> &Self::Target {
        ConstValue::from_nodes(self.nodes.as_slice())
    }
}

impl DerefMut for ConstValueBuf {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let nodes = self.nodes.as_mut_slice();
        // SAFETY: `ConstValue` is `#[repr(transparent)]` over `[ConstNode]`.
        unsafe { &mut *(nodes as *mut [ConstNode] as *mut ConstValue) }
    }
}

/// A constant is written down as the kinds its nodes are, since the shape is
/// worked out from them again when it is read back - so there is nothing for a
/// size written down to disagree with.
#[cfg(feature = "serde")]
impl Serialize for ConstValueBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.nodes.as_slice().len()))?;

        for kind in self.kinds() {
            seq.serialize_element(kind)?;
        }

        seq.end()
    }
}

/// Reading a constant back checks that what was written down is a tree, and
/// that it is one within the limits, before anything walks it.
#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ConstValueBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let kinds = Vec::<ConstNodeKind>::deserialize(deserializer)?;
        ConstValueBuf::from_kinds(kinds).map_err(D::Error::custom)
    }
}

#[cfg(feature = "musli")]
impl<M> Encode<M> for ConstValueBuf
where
    ConstNodeKind: Encode<M>,
{
    type Encode = Self;

    const IS_BITWISE_ENCODE: bool = false;

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: musli_core::en::Encoder<Mode = M>,
    {
        use musli_core::en::{Encoder, SequenceEncoder};

        encoder.encode_sequence_fn(self.nodes.as_slice().len(), |seq| {
            for kind in self.kinds() {
                seq.encode_next()?.encode(kind)?;
            }

            Ok(())
        })
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self
    }
}

#[cfg(feature = "musli")]
impl<'de, M, A> Decode<'de, M, A> for ConstValueBuf
where
    A: musli_core::Allocator,
    ConstNodeKind: Decode<'de, M, A>,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: musli_core::de::Decoder<'de, Mode = M, Allocator = A>,
    {
        use musli_core::Context;

        let cx = decoder.cx();
        let kinds = decoder.decode::<Vec<ConstNodeKind>>()?;
        ConstValueBuf::from_kinds(kinds).map_err(|error| cx.custom(error))
    }
}

impl AsRef<ConstValue> for ConstValueBuf {
    #[inline]
    fn as_ref(&self) -> &ConstValue {
        self
    }
}

impl core::borrow::Borrow<ConstValue> for ConstValueBuf {
    #[inline]
    fn borrow(&self) -> &ConstValue {
        self
    }
}

impl TryClone for ConstValueBuf {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            nodes: self.nodes.try_clone()?,
        })
    }
}

impl fmt::Debug for ConstValueBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

/// Builds the array a constant is made of.
///
/// Nodes are appended in the order they are walked, which is the order they are
/// stored in. A node which has subtrees below it is opened before them and
/// closed after, since how long its subtree is is only known once they are all
/// there.
pub(crate) struct ConstBuilder {
    nodes: Vec<ConstNode>,
    /// The greatest height among the subtrees which are complete at each level
    /// which is still open, innermost last.
    levels: Vec<u32>,
    /// The greatest height among the subtrees which are complete at the
    /// outermost level.
    height: u32,
}

impl ConstBuilder {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            levels: Vec::new(),
            height: 0,
        }
    }

    /// How many nodes have been appended so far, which is what
    /// [`MAX_CONST_SIZE`] is measured against.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Say that a subtree of `height` is complete at the level being built.
    #[inline]
    fn record(&mut self, height: u32) {
        match self.levels.last_mut() {
            Some(level) => *level = (*level).max(height),
            None => self.height = self.height.max(height),
        }
    }

    /// Append a node which has nothing below it.
    #[inline]
    pub(crate) fn leaf(&mut self, kind: ConstNodeKind) -> alloc::Result<()> {
        self.nodes.try_push(ConstNode {
            size: 1,
            height: 1,
            kind,
        })?;

        self.record(1);
        Ok(())
    }

    /// Append a node whose subtrees are appended after it, handing back what
    /// [`ConstBuilder::close`] needs.
    #[inline]
    pub(crate) fn open(&mut self, kind: ConstNodeKind) -> alloc::Result<usize> {
        let at = self.nodes.len();

        self.nodes.try_push(ConstNode {
            size: 0,
            height: 0,
            kind,
        })?;

        self.levels.try_push(0)?;
        Ok(at)
    }

    /// Say that everything below the node opened at `at` has been appended.
    #[inline]
    pub(crate) fn close(&mut self, at: usize) {
        let below = self.levels.pop().unwrap_or(0);
        let height = below.saturating_add(1);
        let size = self.nodes.len().saturating_sub(at);

        if let Some(node) = self.nodes.get_mut(at) {
            node.size = size as u32;
            node.height = height;
        }

        self.record(height);
    }

    /// Append a constant which has already been built, as it is.
    #[inline]
    pub(crate) fn extend(&mut self, value: &ConstValue) -> alloc::Result<()> {
        self.nodes.try_reserve(value.as_nodes().len())?;

        for node in value.as_nodes() {
            self.nodes.try_push(node.try_clone()?)?;
        }

        self.record(value.height());
        Ok(())
    }

    /// Take what has been built, checking that it is within the limits.
    ///
    /// Checking here is what makes the limits hold of *every* constant rather
    /// than only of the ones built from a value: a host which nests one
    /// constant inside another in a loop is bounded the same way a script is.
    pub(crate) fn build(mut self) -> Result<ConstValueBuf, ConstNodesError> {
        if self.nodes.is_empty() {
            self.leaf(ConstNodeKind::Inline(Inline::Empty))?;
        }

        if self.nodes.len() > MAX_CONST_SIZE {
            return Err(ConstNodesError::TooLarge {
                max: MAX_CONST_SIZE,
            });
        }

        if self.height as usize > MAX_CONST_DEPTH {
            return Err(ConstNodesError::TooDeep {
                max: MAX_CONST_DEPTH,
            });
        }

        Ok(ConstValueBuf::from_vec(take(&mut self.nodes))?)
    }
}

/// Formatting a constant, which walks it without recursing into it.
mod debug {
    use core::fmt;

    use crate::alloc::{Box, Vec};
    use crate::Hash;

    use super::{ConstNodeKind, ConstValue};

    /// A node whose subtrees are part way through being written.
    struct Level<'a> {
        /// How many of them are left.
        remaining: usize,
        /// How many there were, so that the one being written can be found.
        total: usize,
        /// What names them, if this is an object.
        keys: Option<&'a [Box<str>]>,
    }

    pub(super) fn fmt(value: &ConstValue, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A constant nests as deeply as whoever built it made it, so the levels
        // which are part way through are kept here rather than as native
        // frames.
        let mut levels = Vec::<Level<'_>>::new();

        for node in value.as_nodes() {
            if let Some(level) = levels.last_mut() {
                let index = level.total - level.remaining;

                if index > 0 {
                    write!(f, ", ")?;
                }

                if let Some(key) = level.keys.and_then(|keys| keys.get(index)) {
                    write!(f, "{key:?}: ")?;
                }

                level.remaining -= 1;
            }

            let level = match &node.kind {
                ConstNodeKind::Inline(value) => {
                    write!(f, "{value:?}")?;
                    None
                }
                ConstNodeKind::String(value) => {
                    write!(f, "{value:?}")?;
                    None
                }
                ConstNodeKind::Bytes(value) => {
                    write!(f, "{value:?}")?;
                    None
                }
                ConstNodeKind::Instance {
                    hash,
                    variant_hash,
                    fields,
                } => {
                    if *variant_hash == Hash::EMPTY {
                        write!(f, "{hash}(")?;
                    } else {
                        write!(f, "{hash}::{variant_hash}(")?;
                    }

                    Some(Level {
                        remaining: *fields as usize,
                        total: *fields as usize,
                        keys: None,
                    })
                }
                ConstNodeKind::Object { keys } => {
                    write!(f, "#{{")?;

                    Some(Level {
                        remaining: keys.len(),
                        total: keys.len(),
                        keys: Some(keys),
                    })
                }
            };

            if let Some(level) = level {
                levels.try_push(level).map_err(|_| fmt::Error)?;
            }

            // Close every level which the node just written completed,
            // innermost first.
            while levels.last().is_some_and(|level| level.remaining == 0) {
                let Some(level) = levels.pop() else {
                    break;
                };

                if level.keys.is_some() {
                    write!(f, "}}")?;
                } else {
                    write!(f, ")")?;
                }
            }
        }

        Ok(())
    }
}
