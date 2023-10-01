#[cfg(test)]
mod tests;

use core::alloc::Layout;
use core::cell::{Cell, RefCell};
use core::marker::PhantomData;
use core::mem;
use core::ptr;
use core::slice;
use core::str;

use crate::alloc::{self, try_vec, Box, HashMap, Vec};

#[non_exhaustive]
pub struct ArenaWriteSliceOutOfBounds {
    pub index: usize,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct ArenaAllocError {
    pub requested: usize,
}

impl From<alloc::Error> for ArenaAllocError {
    fn from(_: alloc::Error) -> Self {
        Self { requested: 0 }
    }
}

/// The size of a slab in the arena allocator.
const PAGE: usize = 4096;
const HUGE_PAGE: usize = 2 * 1024 * 1024;

struct Chunk {
    storage: Box<[u8]>,
}

impl Chunk {
    /// Construct a new chunk with the specified length.
    fn new(len: usize) -> Result<Self, ArenaAllocError> {
        Ok(Self {
            storage: try_vec![0u8; len].try_into_boxed_slice()?,
        })
    }
}

/// An arena allocator.
pub struct Arena {
    start: Cell<*mut u8>,
    end: Cell<*mut u8>,
    chunks: RefCell<Vec<Chunk>>,
    /// Allocated bytes. The pointers are stable into the chunks.
    bytes: RefCell<HashMap<Box<[u8]>, ptr::NonNull<u8>>>,
}

impl Arena {
    /// Construct a new empty arena allocator.
    pub fn new() -> Self {
        Self {
            start: Cell::new(ptr::null_mut()),
            end: Cell::new(ptr::null_mut()),
            chunks: RefCell::new(Vec::new()),
            bytes: RefCell::new(HashMap::new()),
        }
    }

    /// Allocate a string from the arena.
    pub(crate) fn alloc_bytes(&self, bytes: &[u8]) -> Result<&[u8], ArenaAllocError> {
        if let Some(ptr) = self.bytes.borrow().get(bytes).copied() {
            // SAFETY: The pointer returned was previously allocated correctly
            // in the arena.
            unsafe {
                return Ok(slice::from_raw_parts(ptr.as_ptr() as *const _, bytes.len()));
            }
        }

        let layout = Layout::array::<u8>(bytes.len()).map_err(|_| ArenaAllocError {
            requested: bytes.len(),
        })?;
        let ptr = self.alloc_raw(layout)?;

        // SAFETY: we're ensuring the valid contents of pointer by copying a
        // safe bytes slice into it.
        let output = unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), bytes.len());
            slice::from_raw_parts(ptr.as_ptr() as *const _, bytes.len())
        };

        self.bytes.borrow_mut().try_insert(bytes.try_into()?, ptr)?;
        Ok(output)
    }

    /// Allocate a string from the arena.
    pub(crate) fn alloc_str(&self, string: &str) -> Result<&str, ArenaAllocError> {
        let bytes = self.alloc_bytes(string.as_bytes())?;

        // SAFETY: we're ensuring the valid contents of the returned string by
        // safely accessing it above.
        unsafe {
            return Ok(str::from_utf8_unchecked(bytes));
        }
    }

    /// Allocate a new object of the given type.
    pub(crate) fn alloc<T>(&self, object: T) -> Result<&mut T, ArenaAllocError> {
        assert!(!mem::needs_drop::<T>());

        let mut ptr = self.alloc_raw(Layout::for_value::<T>(&object))?.cast();

        unsafe {
            // Write into uninitialized memory.
            ptr::write(ptr.as_ptr(), object);
            Ok(ptr.as_mut())
        }
    }

    /// Allocate an iterator with the given length as a slice.
    pub(crate) fn alloc_iter<T>(&self, len: usize) -> Result<AllocIter<'_, T>, ArenaAllocError> {
        assert!(!mem::needs_drop::<T>(), "cannot allocate drop element");

        let mem = if len == 0 {
            None
        } else {
            Some(self.alloc_raw(Layout::array::<T>(len).unwrap())?.cast())
        };

        Ok(AllocIter {
            mem,
            index: 0,
            len,
            _marker: PhantomData,
        })
    }

    #[inline]
    fn alloc_raw_without_grow(&self, layout: Layout) -> Option<ptr::NonNull<u8>> {
        let start = addr(self.start.get());
        let old_end = self.end.get();
        let end = addr(old_end);

        let align = layout.align();
        let bytes = layout.size();

        let new_end = end.checked_sub(bytes)? & !(align - 1);

        if start > new_end {
            return None;
        }

        let new_end = with_addr(old_end, new_end);
        self.end.set(new_end);

        // Pointer is guaranteed to be non-null due to how it's allocated.
        unsafe { Some(ptr::NonNull::new_unchecked(new_end)) }
    }

    #[inline]
    fn alloc_raw(&self, layout: Layout) -> Result<ptr::NonNull<u8>, ArenaAllocError> {
        // assert!(layout.size() != 0);
        assert!(layout.align() != 0);

        if layout.size() == 0 {
            // SAFETY: we've asserted that alignment is non-zero above.
            return unsafe { Ok(ptr::NonNull::new_unchecked(layout.align() as *mut u8)) };
        }

        loop {
            if let Some(a) = self.alloc_raw_without_grow(layout) {
                break Ok(a);
            }

            self.grow(layout.size())?;
        }
    }

    #[cold]
    fn grow(&self, additional: usize) -> Result<(), ArenaAllocError> {
        let mut chunks = self.chunks.borrow_mut();

        let new_cap = additional.max(
            chunks
                .last()
                .map(|c| c.storage.len().min(HUGE_PAGE / 2) * 2)
                .unwrap_or(PAGE),
        );

        chunks.try_push(Chunk::new(new_cap)?)?;

        let Some(chunk) = chunks.last_mut() else {
            return Err(ArenaAllocError {
                requested: additional,
            });
        };

        let range = chunk.storage.as_mut_ptr_range();
        self.start.set(range.start);
        self.end.set(range.end);
        Ok(())
    }
}

#[inline]
pub(crate) fn addr(this: *mut u8) -> usize {
    this as usize
}

#[inline]
pub(crate) fn with_addr(this: *mut u8, a: usize) -> *mut u8 {
    let this_addr = addr(this) as isize;
    let dest_addr = a as isize;
    let offset = dest_addr.wrapping_sub(this_addr);
    this.wrapping_offset(offset)
}

/// An iterator writer.
pub(crate) struct AllocIter<'hir, T> {
    mem: Option<ptr::NonNull<T>>,
    index: usize,
    len: usize,
    _marker: PhantomData<&'hir ()>,
}

impl<'hir, T> AllocIter<'hir, T> {
    /// Write the next element into the slice.
    pub(crate) fn write(&mut self, object: T) -> Result<(), ArenaWriteSliceOutOfBounds> {
        let mem = self
            .mem
            .ok_or(ArenaWriteSliceOutOfBounds { index: self.index })?;

        // Sanity check is necessary to ensure memory safety.
        if self.index >= self.len {
            return Err(ArenaWriteSliceOutOfBounds { index: self.index });
        }

        unsafe {
            ptr::write(mem.as_ptr().add(self.index), object);
            self.index += 1;
            Ok(())
        }
    }

    /// Finalize the iterator being written and return the appropriate closure.
    pub(crate) fn finish(self) -> &'hir mut [T] {
        match self.mem {
            Some(mem) => {
                // SAFETY: Is guaranteed to be correct due to how it's allocated and written to above.
                unsafe { slice::from_raw_parts_mut(mem.as_ptr(), self.index) }
            }
            None => &mut [],
        }
    }
}
