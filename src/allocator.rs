//! Allocateur mémoire pour environnement no_std (bump allocator)

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 1024 * 1024; // 1 MB

#[repr(C, align(4096))]
struct HeapMemory {
    data: [u8; HEAP_SIZE],
}

static mut HEAP: HeapMemory = HeapMemory {
    data: [0; HEAP_SIZE],
};

static HEAP_POS: AtomicUsize = AtomicUsize::new(0);

/// Bump Allocator - allocateur simple qui avance un pointeur
pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    /// Alloue de la mémoire
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();

        loop {
            let current = HEAP_POS.load(Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_pos = aligned + size;

            if new_pos > HEAP_SIZE {
                return null_mut();
            }

            match HEAP_POS.compare_exchange_weak(
                current,
                new_pos,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return HEAP.data.as_mut_ptr().add(aligned);
                }
                Err(_) => continue,
            }
        }
    }

    /// Désalloue (no-op pour bump allocator)
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}

    /// Réalloue de la mémoire
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);

        if !new_ptr.is_null() {
            let copy_size = layout.size().min(new_size);
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        }

        new_ptr
    }
}

// Décommenter pour no_std:
// #[global_allocator]
// static ALLOCATOR: BumpAllocator = BumpAllocator;

/// Retourne l'utilisation actuelle du heap
pub fn heap_usage() -> usize {
    HEAP_POS.load(Ordering::Relaxed)
}

/// Retourne l'espace restant du heap
pub fn heap_remaining() -> usize {
    HEAP_SIZE - heap_usage()
}

/// Retourne la taille totale du heap
pub const fn heap_size() -> usize {
    HEAP_SIZE
}

/// Reset l'allocateur (pour tests uniquement)
#[cfg(test)]
pub unsafe fn reset_allocator() {
    HEAP_POS.store(0, Ordering::SeqCst);
}

// Linked List Allocator (alternative plus complexe)

#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

/// Allocateur à liste chaînée (supporte la désallocation)
pub struct LinkedListAllocator {
    head: AtomicUsize,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        LinkedListAllocator {
            head: AtomicUsize::new(0),
        }
    }

    /// Initialise l'allocateur avec une région mémoire
    pub unsafe fn init(&self, start: *mut u8, size: usize) {
        let block = start as *mut FreeBlock;
        (*block).size = size;
        (*block).next = null_mut();
        self.head.store(block as usize, Ordering::SeqCst);
    }

    /// Alloue de la mémoire
    pub unsafe fn allocate(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let align = layout.align().max(core::mem::align_of::<FreeBlock>());

        let mut prev: *mut FreeBlock = null_mut();
        let mut current = self.head.load(Ordering::Acquire) as *mut FreeBlock;

        while !current.is_null() {
            let block_start = current as usize;
            let aligned_start = (block_start + align - 1) & !(align - 1);
            let padding = aligned_start - block_start;
            let total_size = padding + size;

            if (*current).size >= total_size {
                let remaining = (*current).size - total_size;

                if remaining >= core::mem::size_of::<FreeBlock>() {
                    let new_block = (aligned_start + size) as *mut FreeBlock;
                    (*new_block).size = remaining;
                    (*new_block).next = (*current).next;

                    if prev.is_null() {
                        self.head.store(new_block as usize, Ordering::Release);
                    } else {
                        (*prev).next = new_block;
                    }
                } else {
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

        null_mut()
    }

    /// Désalloue de la mémoire
    pub unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(core::mem::size_of::<FreeBlock>());
        let block = ptr as *mut FreeBlock;
        (*block).size = size;

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

    #[test]
    fn test_alignment() {
        let b1 = Box::new(1u8);
        let b2 = Box::new(1u64);

        let ptr = &*b2 as *const u64 as usize;
        assert_eq!(ptr % 8, 0);

        drop(b1);
        drop(b2);
    }
}
