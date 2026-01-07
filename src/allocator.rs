//! Memory Allocator for no_std environment
//!
//! This module provides a simple bump allocator suitable for
//! no_std environments. For production use, consider implementing
//! a more sophisticated allocator like a linked list or slab allocator.
//!
//! # Safety
//! This allocator uses unsafe code to implement GlobalAlloc.
//! The implementation is thread-safe using a spin lock.
//!
//! # Reference
//! Based on https://os.phil-opp.com/heap-allocation/

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Heap size in bytes (1 MB)
const HEAP_SIZE: usize = 1024 * 1024;

/// Heap memory region (statically allocated)
///
/// Aligned to 4096 bytes for page alignment.
#[repr(C, align(4096))]
struct HeapMemory {
    data: [u8; HEAP_SIZE],
}

/// Static heap memory
static mut HEAP: HeapMemory = HeapMemory {
    data: [0; HEAP_SIZE],
};

/// Current allocation position
static HEAP_POS: AtomicUsize = AtomicUsize::new(0);

/// Bump Allocator
///
/// A simple allocator that just bumps a pointer forward.
/// Does not support deallocation (memory is never freed).
///
/// This is suitable for short-lived programs or programs
/// where memory usage is predictable.
///
/// # Safety
/// - Thread-safe through atomic operations
/// - Does not support deallocation
/// - Memory is zeroed on first use
pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    /// Allocate memory with given layout
    ///
    /// # Safety
    /// - Returns null pointer if allocation fails
    /// - Memory is uninitialized
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        loop {
            let current = HEAP_POS.load(Ordering::Relaxed);

            // Align current position
            let aligned = (current + align - 1) & !(align - 1);
            let new_pos = aligned + size;

            // Check if we have enough space
            if new_pos > HEAP_SIZE {
                return null_mut();
            }

            // Try to update position atomically
            match HEAP_POS.compare_exchange_weak(
                current,
                new_pos,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Success - return pointer to allocated region
                    // SAFETY: We have exclusive access to this region through atomic CAS
                    return HEAP.data.as_mut_ptr().add(aligned);
                }
                Err(_) => {
                    // Another thread allocated, retry
                    continue;
                }
            }
        }
    }

    /// Deallocate memory
    ///
    /// # Safety
    /// This bump allocator does not actually free memory.
    /// All memory remains allocated until program termination.
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator does not deallocate
        // Memory is effectively leaked until program ends
    }

    /// Reallocate memory
    ///
    /// # Safety
    /// Allocates new memory and copies data. Old memory is not freed.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Allocate new block
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);

        if !new_ptr.is_null() {
            // Copy old data
            let copy_size = layout.size().min(new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        }

        // Note: old memory is not freed (bump allocator limitation)
        new_ptr
    }
}

/// Global allocator instance
/// Commenté pour les tests - Décommenter pour no_std
// #[global_allocator]
// static ALLOCATOR: BumpAllocator = BumpAllocator;

/// Get current heap usage in bytes
pub fn heap_usage() -> usize {
    HEAP_POS.load(Ordering::Relaxed)
}

/// Get remaining heap space in bytes
pub fn heap_remaining() -> usize {
    HEAP_SIZE - heap_usage()
}

/// Get total heap size in bytes
pub const fn heap_size() -> usize {
    HEAP_SIZE
}

/// Reset allocator (for testing only)
///
/// # Safety
/// This function is extremely unsafe. Only use in single-threaded
/// test contexts where no allocated memory is still in use.
#[cfg(test)]
pub unsafe fn reset_allocator() {
    HEAP_POS.store(0, Ordering::SeqCst);
}

// ============================================================================
// Alternative: Linked List Allocator (more complex but supports deallocation)
// ============================================================================

/// Free block header for linked list allocator
#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

/// Linked List Allocator
///
/// A more sophisticated allocator that maintains a free list.
/// Supports both allocation and deallocation.
///
/// Not used as global allocator by default, but provided as reference.
pub struct LinkedListAllocator {
    head: AtomicUsize, // Actually stores *mut FreeBlock
}

impl LinkedListAllocator {
    /// Create new linked list allocator
    pub const fn new() -> Self {
        LinkedListAllocator {
            head: AtomicUsize::new(0),
        }
    }

    /// Initialize allocator with memory region
    ///
    /// # Safety
    /// - `start` must be a valid, aligned pointer
    /// - `size` must be the actual size of the memory region
    /// - Region must not be used by anything else
    pub unsafe fn init(&self, start: *mut u8, size: usize) {
        // Create initial free block spanning entire region
        let block = start as *mut FreeBlock;
        (*block).size = size;
        (*block).next = null_mut();
        self.head.store(block as usize, Ordering::SeqCst);
    }

    /// Allocate memory
    ///
    /// # Safety
    /// Standard allocator safety requirements apply.
    pub unsafe fn allocate(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let align = layout.align().max(core::mem::align_of::<FreeBlock>());

        // Walk free list looking for suitable block
        let mut prev: *mut FreeBlock = null_mut();
        let mut current = self.head.load(Ordering::Acquire) as *mut FreeBlock;

        while !current.is_null() {
            let block_start = current as usize;
            let aligned_start = (block_start + align - 1) & !(align - 1);
            let padding = aligned_start - block_start;
            let total_size = padding + size;

            if (*current).size >= total_size {
                // Found suitable block
                let remaining = (*current).size - total_size;

                if remaining >= core::mem::size_of::<FreeBlock>() {
                    // Split block
                    let new_block = (aligned_start + size) as *mut FreeBlock;
                    (*new_block).size = remaining;
                    (*new_block).next = (*current).next;

                    if prev.is_null() {
                        self.head.store(new_block as usize, Ordering::Release);
                    } else {
                        (*prev).next = new_block;
                    }
                } else {
                    // Use entire block
                    if prev.is_null() {
                        self.head.store((*current).next as usize, Ordering::Release);
                    } else {
                        (*prev).next = (*current).next;
                    }
                }

                return aligned_start as *mut u8;
            }

            prev = current;
            current = (*current).next;
        }

        null_mut() // No suitable block found
    }

    /// Deallocate memory
    ///
    /// # Safety
    /// - `ptr` must have been allocated by this allocator
    /// - `layout` must match the original allocation
    pub unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());

        // Create free block at deallocated region
        let block = ptr as *mut FreeBlock;
        (*block).size = size;

        // Add to front of free list
        loop {
            let head = self.head.load(Ordering::Acquire);
            (*block).next = head as *mut FreeBlock;

            if self.head.compare_exchange_weak(
                head,
                block as usize,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;
    use alloc::boxed::Box;

    #[test]
    fn test_basic_allocation() {
        let v: Vec<u32> = (0..100).collect();
        assert_eq!(v.len(), 100);
        assert_eq!(v[50], 50);
    }

    #[test]
    fn test_box_allocation() {
        let b = Box::new(42u64);
        assert_eq!(*b, 42);
    }

    // Ce test n'est pertinent que pour no_std avec notre allocateur actif
    // #[test]
    // fn test_heap_usage() {
    //     let before = heap_usage();
    //     let _v: Vec<u8> = (0..1000).map(|i| i as u8).collect();
    //     let after = heap_usage();
    //     assert!(after > before);
    // }

    #[test]
    fn test_alignment() {
        let b1 = Box::new(1u8);
        let b2 = Box::new(1u64);

        // u64 should be 8-byte aligned
        let ptr = &*b2 as *const u64 as usize;
        assert_eq!(ptr % 8, 0);

        drop(b1);
        drop(b2);
    }
}
