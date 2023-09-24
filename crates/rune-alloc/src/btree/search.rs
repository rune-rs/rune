use core::borrow::Borrow;
use core::cmp::Ordering;
use core::ops::{Bound, RangeBounds};

use super::node::{marker, ForceResult::*, Handle, NodeRef};

use SearchBound::*;
use SearchResult::*;

pub(crate) enum SearchBound<T> {
    /// An inclusive bound to look for, just like `Bound::Included(T)`.
    Included(T),
    /// An exclusive bound to look for, just like `Bound::Excluded(T)`.
    Excluded(T),
    /// An unconditional inclusive bound, just like `Bound::Unbounded`.
    AllIncluded,
    /// An unconditional exclusive bound.
    AllExcluded,
}

impl<T> SearchBound<T> {
    pub(crate) fn from_range(range_bound: Bound<T>) -> Self {
        match range_bound {
            Bound::Included(t) => Included(t),
            Bound::Excluded(t) => Excluded(t),
            Bound::Unbounded => AllIncluded,
        }
    }
}

pub(crate) enum SearchResult<BorrowType, K, V, FoundType, GoDownType> {
    Found(Handle<NodeRef<BorrowType, K, V, FoundType>, marker::KV>),
    GoDown(Handle<NodeRef<BorrowType, K, V, GoDownType>, marker::Edge>),
}

pub(crate) enum IndexResult {
    KV(usize),
    Edge(usize),
}

impl<BorrowType: marker::BorrowType, K, V> NodeRef<BorrowType, K, V, marker::LeafOrInternal> {
    /// Looks up a given key in a (sub)tree headed by the node, recursively.
    /// Returns a `Found` with the handle of the matching KV, if any. Otherwise,
    /// returns a `GoDown` with the handle of the leaf edge where the key belongs.
    ///
    /// The result is meaningful only if the tree is ordered by key, like the tree
    /// in a `BTreeMap` is.
    pub(crate) fn search_tree<C: ?Sized, Q: ?Sized, E>(
        mut self,
        cx: &mut C,
        key: &Q,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<SearchResult<BorrowType, K, V, marker::LeafOrInternal, marker::Leaf>, E>
    where
        K: Borrow<Q>,
    {
        loop {
            self = match self.search_node(cx, key, cmp)? {
                Found(handle) => return Ok(Found(handle)),
                GoDown(handle) => match handle.force() {
                    Leaf(leaf) => return Ok(GoDown(leaf)),
                    Internal(internal) => internal.descend(),
                },
            }
        }
    }

    /// Descends to the nearest node where the edge matching the lower bound
    /// of the range is different from the edge matching the upper bound, i.e.,
    /// the nearest node that has at least one key contained in the range.
    ///
    /// If found, returns an `Ok` with that node, the strictly ascending pair of
    /// edge indices in the node delimiting the range, and the corresponding
    /// pair of bounds for continuing the search in the child nodes, in case
    /// the node is internal.
    ///
    /// If not found, returns an `Err` with the leaf edge matching the entire
    /// range.
    ///
    /// As a diagnostic service, panics if the range specifies impossible bounds.
    ///
    /// The result is meaningful only if the tree is ordered by key.
    pub(crate) fn search_tree_for_bifurcation<'r, C: ?Sized, Q: ?Sized, R, E>(
        mut self,
        cx: &mut C,
        range: &'r R,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<
        Result<
            (
                NodeRef<BorrowType, K, V, marker::LeafOrInternal>,
                usize,
                usize,
                SearchBound<&'r Q>,
                SearchBound<&'r Q>,
            ),
            Handle<NodeRef<BorrowType, K, V, marker::Leaf>, marker::Edge>,
        >,
        E,
    >
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
    {
        // Inlining these variables should be avoided. We assume the bounds reported by `range`
        // remain the same, but an adversarial implementation could change between calls (#81138).
        let (start, end) = (range.start_bound(), range.end_bound());
        match (start, end) {
            (Bound::Excluded(s), Bound::Excluded(e))
                if matches!(cmp(cx, s, e)?, Ordering::Equal) =>
            {
                panic!("range start and end are equal and excluded in BTree")
            }
            (Bound::Included(s) | Bound::Excluded(s), Bound::Included(e) | Bound::Excluded(e))
                if matches!(cmp(cx, s, e)?, Ordering::Greater) =>
            {
                panic!("range start is greater than range end in BTree")
            }
            _ => {}
        }
        let mut lower_bound = SearchBound::from_range(start);
        let mut upper_bound = SearchBound::from_range(end);
        loop {
            let (lower_edge_idx, lower_child_bound) =
                self.find_lower_bound_index(cx, lower_bound, cmp)?;
            let (upper_edge_idx, upper_child_bound) =
                unsafe { self.find_upper_bound_index(cx, upper_bound, lower_edge_idx, cmp)? };
            if lower_edge_idx < upper_edge_idx {
                return Ok(Ok((
                    self,
                    lower_edge_idx,
                    upper_edge_idx,
                    lower_child_bound,
                    upper_child_bound,
                )));
            }
            debug_assert_eq!(lower_edge_idx, upper_edge_idx);
            let common_edge = unsafe { Handle::new_edge(self, lower_edge_idx) };
            match common_edge.force() {
                Leaf(common_edge) => return Ok(Err(common_edge)),
                Internal(common_edge) => {
                    self = common_edge.descend();
                    lower_bound = lower_child_bound;
                    upper_bound = upper_child_bound;
                }
            }
        }
    }

    /// Finds an edge in the node delimiting the lower bound of a range.
    /// Also returns the lower bound to be used for continuing the search in
    /// the matching child node, if `self` is an internal node.
    ///
    /// The result is meaningful only if the tree is ordered by key.
    pub(crate) fn find_lower_bound_edge<'r, C: ?Sized, Q: ?Sized, E>(
        self,
        cx: &mut C,
        bound: SearchBound<&'r Q>,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<(Handle<Self, marker::Edge>, SearchBound<&'r Q>), E>
    where
        K: Borrow<Q>,
    {
        let (edge_idx, bound) = self.find_lower_bound_index(cx, bound, cmp)?;
        let edge = unsafe { Handle::new_edge(self, edge_idx) };
        Ok((edge, bound))
    }

    /// Clone of `find_lower_bound_edge` for the upper bound.
    pub(crate) fn find_upper_bound_edge<'r, C: ?Sized, Q: ?Sized, E>(
        self,
        cx: &mut C,
        bound: SearchBound<&'r Q>,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<(Handle<Self, marker::Edge>, SearchBound<&'r Q>), E>
    where
        K: Borrow<Q>,
    {
        let (edge_idx, bound) = unsafe { self.find_upper_bound_index(cx, bound, 0, cmp)? };
        let edge = unsafe { Handle::new_edge(self, edge_idx) };
        Ok((edge, bound))
    }
}

impl<BorrowType, K, V, Type> NodeRef<BorrowType, K, V, Type> {
    /// Looks up a given key in the node, without recursion.
    /// Returns a `Found` with the handle of the matching KV, if any. Otherwise,
    /// returns a `GoDown` with the handle of the edge where the key might be found
    /// (if the node is internal) or where the key can be inserted.
    ///
    /// The result is meaningful only if the tree is ordered by key, like the tree
    /// in a `BTreeMap` is.
    pub(crate) fn search_node<C: ?Sized, Q: ?Sized, E>(
        self,
        cx: &mut C,
        key: &Q,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<SearchResult<BorrowType, K, V, Type, Type>, E>
    where
        K: Borrow<Q>,
    {
        Ok(match unsafe { self.find_key_index(cx, key, 0, cmp)? } {
            IndexResult::KV(idx) => Found(unsafe { Handle::new_kv(self, idx) }),
            IndexResult::Edge(idx) => GoDown(unsafe { Handle::new_edge(self, idx) }),
        })
    }

    /// Returns either the KV index in the node at which the key (or an equivalent)
    /// exists, or the edge index where the key belongs, starting from a particular index.
    ///
    /// The result is meaningful only if the tree is ordered by key, like the tree
    /// in a `BTreeMap` is.
    ///
    /// # Safety
    /// `start_index` must be a valid edge index for the node.
    unsafe fn find_key_index<C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        key: &Q,
        start_index: usize,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<IndexResult, E>
    where
        K: Borrow<Q>,
    {
        let node = self.reborrow();
        let keys = node.keys();
        debug_assert!(start_index <= keys.len());
        for (offset, k) in unsafe { keys.get_unchecked(start_index..) }
            .iter()
            .enumerate()
        {
            match cmp(cx, key, k.borrow())? {
                Ordering::Greater => {}
                Ordering::Equal => return Ok(IndexResult::KV(start_index + offset)),
                Ordering::Less => return Ok(IndexResult::Edge(start_index + offset)),
            }
        }
        Ok(IndexResult::Edge(keys.len()))
    }

    /// Finds an edge index in the node delimiting the lower bound of a range.
    /// Also returns the lower bound to be used for continuing the search in
    /// the matching child node, if `self` is an internal node.
    ///
    /// The result is meaningful only if the tree is ordered by key.
    fn find_lower_bound_index<'r, C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        bound: SearchBound<&'r Q>,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<(usize, SearchBound<&'r Q>), E>
    where
        K: Borrow<Q>,
    {
        Ok(match bound {
            Included(key) => match unsafe { self.find_key_index(cx, key, 0, cmp)? } {
                IndexResult::KV(idx) => (idx, AllExcluded),
                IndexResult::Edge(idx) => (idx, bound),
            },
            Excluded(key) => match unsafe { self.find_key_index(cx, key, 0, cmp)? } {
                IndexResult::KV(idx) => (idx + 1, AllIncluded),
                IndexResult::Edge(idx) => (idx, bound),
            },
            AllIncluded => (0, AllIncluded),
            AllExcluded => (self.len(), AllExcluded),
        })
    }

    /// Mirror image of `find_lower_bound_index` for the upper bound,
    /// with an additional parameter to skip part of the key array.
    ///
    /// # Safety
    /// `start_index` must be a valid edge index for the node.
    unsafe fn find_upper_bound_index<'r, C: ?Sized, Q: ?Sized, E>(
        &self,
        cx: &mut C,
        bound: SearchBound<&'r Q>,
        start_index: usize,
        cmp: fn(&mut C, &Q, &Q) -> Result<Ordering, E>,
    ) -> Result<(usize, SearchBound<&'r Q>), E>
    where
        K: Borrow<Q>,
    {
        Ok(match bound {
            Included(key) => match unsafe { self.find_key_index(cx, key, start_index, cmp)? } {
                IndexResult::KV(idx) => (idx + 1, AllExcluded),
                IndexResult::Edge(idx) => (idx, bound),
            },
            Excluded(key) => match unsafe { self.find_key_index(cx, key, start_index, cmp)? } {
                IndexResult::KV(idx) => (idx, AllIncluded),
                IndexResult::Edge(idx) => (idx, bound),
            },
            AllIncluded => (self.len(), AllIncluded),
            AllExcluded => (start_index, AllExcluded),
        })
    }
}
