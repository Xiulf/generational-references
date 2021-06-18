#![no_std]

extern crate alloc;
extern crate core;

use alloc::alloc::{alloc, Layout};
use alloc::vec::Vec;
use core::mem::size_of;
use core::ptr::NonNull;
use core::ops::{Deref, DerefMut};

static mut FREE_LIST: Vec<FreeAllocation> = Vec::new();

struct FreeAllocation {
    size: usize,
    ptr: *mut u8,
    gen: usize,
}

#[repr(C)]
pub struct GenAllocResult<T> {
    pub ptr: *mut T,
    pub gen: usize,
}

pub unsafe extern "C" fn gen_alloc<T>() -> GenAllocResult<T> {
    let ptr_size = size_of::<usize>();
    let size = ptr_size + size_of::<T>().next_power_of_two();

    if let Some(entry) = FREE_LIST.iter().find(|f| f.size == size) {
        GenAllocResult {
            ptr: entry.ptr as *mut T,
            gen: entry.gen,
        }
    } else {
        let align = alignment(size);
        let layout = Layout::from_size_align(size, align).unwrap();
        let ptr = alloc(layout);

        *(ptr as *mut usize) = 0;

        GenAllocResult {
            ptr: ptr.add(ptr_size) as *mut T,
            gen: 0,
        }
    }
}

pub unsafe extern "C" fn gen_free<T>(ptr: *mut T) {
    let ptr_size = size_of::<usize>();
    let size = ptr_size + size_of::<T>().next_power_of_two();
    let start = (ptr as *mut u8).sub(ptr_size);
    
    *(start as *mut usize) += 1;

    FREE_LIST.push(FreeAllocation {
        size,
        ptr: ptr as *mut u8,
        gen: *(start as *mut usize),
    });
}

pub unsafe extern "C" fn get_generation<T>(ptr: *mut T) -> usize {
    let ptr_size = size_of::<usize>();
    let start = (ptr as *mut u8).sub(ptr_size);

    *(start as *mut usize)
}

fn alignment(mut size: usize) -> usize {
    if size == 0 {
        return 1;
    }

    let mut pow2 = 0;

    while (size & 1) == 0 {
        pow2 += 1;
        size >>= 1;
    }

    pow2
}

#[repr(C)]
pub struct GenOwned<T> {
    ptr: NonNull<T>,
    gen: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GenRef<T> {
    ptr: NonNull<T>,
    gen: usize,
}

impl<T> GenOwned<T> {
    pub fn new(t: T) -> Self {
        let (ptr, gen) = unsafe {
            let GenAllocResult { ptr, gen } = gen_alloc::<T>();

            *ptr = t;

            (NonNull::new_unchecked(ptr), gen)
        };

        GenOwned {
            ptr,
            gen,
        }
    }

    pub fn ptr_eq(self, other: Self) -> bool {
        self.ptr == other.ptr && self.gen == other.gen
    }

    #[inline]
    pub fn as_ref(self) -> GenRef<T> {
        GenRef {
            ptr: self.ptr,
            gen: self.gen,
        }
    }

    pub fn assert_alive(&self) {
        let gen = unsafe { get_generation(self.ptr.as_ptr()) };

        if gen != self.gen {
            panic!("attempt to dereference a freed reference");
        }
    }
}

impl<T> GenRef<T> {
    pub fn ptr_eq(self, other: Self) -> bool {
        self.ptr == other.ptr && self.gen == other.gen
    }

    pub fn assert_alive(&self) {
        let gen = unsafe { get_generation(self.ptr.as_ptr()) };

        if gen != self.gen {
            panic!("attempt to dereference a freed reference");
        }
    }
}

impl<T> Drop for GenOwned<T> {
    fn drop(&mut self) {
        unsafe { gen_free(self.ptr.as_ptr()); }
    }
}

impl<T> Deref for GenOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.assert_alive();
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl<T> DerefMut for GenOwned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.assert_alive();
        unsafe { &mut *self.ptr.as_ptr() }
    }
}

impl<T> Deref for GenRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.assert_alive();
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl<T> DerefMut for GenRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.assert_alive();
        unsafe { &mut *self.ptr.as_ptr() }
    }
}
