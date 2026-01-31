#![doc = include_str!("../README.md")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

// ==============
// === Hunger ===
// ==============

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hunger {
    Full,
    Hungry,
    Starving,
    Devouring,
    Insatiable,
}

// ====================
// === Memory Slots ===
// ====================

const MAX_TRACKED: usize = 65_536;
const EMPTY: usize = usize::MAX;

// === Slot ===

struct Slot {
    addr: AtomicUsize,
    size: AtomicUsize,
}

static REGISTRY: [Slot; MAX_TRACKED] = {
    #[allow(clippy::declare_interior_mutable_const)]
    const EMPTY_SLOT: Slot = Slot {
        addr: AtomicUsize::new(0),
        size: AtomicUsize::new(0),
    };
    [EMPTY_SLOT; MAX_TRACKED]
};

// === Active set ===

// ACTIVE[0..ACTIVE_LEN) are valid slot indices
static ACTIVE: [AtomicUsize; MAX_TRACKED] =
    [const { AtomicUsize::new(EMPTY) }; MAX_TRACKED];
static ACTIVE_LEN: AtomicUsize = AtomicUsize::new(0);

// === Free list (FILO) ===

static FREE: [AtomicUsize; MAX_TRACKED] =
    [const { AtomicUsize::new(EMPTY) }; MAX_TRACKED];
static FREE_TOP: AtomicUsize = AtomicUsize::new(0);

// === Eater control ===

static EVENTS: AtomicUsize = AtomicUsize::new(0);
static EATER_STARTED: AtomicBool = AtomicBool::new(false);

// === Slot allocation / free ===

#[inline(always)]
fn alloc_slot() -> Option<usize> {
    // Reuse from FREE stack (FILO).
    let top = FREE_TOP.load(Ordering::Relaxed);
    if top > 0 {
        let idx = FREE_TOP.fetch_sub(1, Ordering::AcqRel) - 1;
        let slot = FREE[idx].load(Ordering::Acquire);
        if slot != EMPTY {
            return Some(slot);
        }
    }

    // Grow ACTIVE set.
    let len = ACTIVE_LEN.fetch_add(1, Ordering::AcqRel);
    if len < MAX_TRACKED {
        ACTIVE[len].store(len, Ordering::Release);
        Some(len)
    } else {
        ACTIVE_LEN.fetch_sub(1, Ordering::Relaxed);
        None
    }
}

#[inline(always)]
fn free_slot(slot: usize) {
    REGISTRY[slot].addr.store(0, Ordering::Release);
    REGISTRY[slot].size.store(0, Ordering::Relaxed);

    let idx = FREE_TOP.fetch_add(1, Ordering::AcqRel);
    if idx < MAX_TRACKED {
        FREE[idx].store(slot, Ordering::Release);
    }
}

// === Allocator ===

#[derive(Clone, Copy, Debug)]
pub struct Allocator {
    pub hunger: Hunger,
}

impl Allocator {
    #[inline(always)]
    fn first_bite_offset(&self) -> Duration {
        let ms = match self.hunger {
            Hunger::Full => u64::MAX,
            Hunger::Hungry => 1000,
            Hunger::Starving => 0,
            Hunger::Devouring => 0,
            Hunger::Insatiable => 0,
        };
        Duration::from_millis(ms)
    }

    #[inline(always)]
    fn bite_offset(&self) -> Duration {
        let ms = match self.hunger {
            Hunger::Full => u64::MAX,
            Hunger::Hungry => 1000,
            Hunger::Starving => 200,
            Hunger::Devouring => 50,
            Hunger::Insatiable => 10,
        };
        Duration::from_millis(ms)
    }

    #[inline(always)]
    fn corruption_shape(self, n: usize) -> (usize, u64) {
        let words = match self.hunger {
            Hunger::Full => 0,
            Hunger::Hungry => 1,
            Hunger::Starving => 2,
            Hunger::Devouring => 4,
            Hunger::Insatiable => 8,
        };

        let mask = match self.hunger {
            Hunger::Full => 0,
            Hunger::Hungry => 1u64 << (n & 1),
            Hunger::Starving => 0b11,
            Hunger::Devouring => 0b111,
            Hunger::Insatiable => 0xFF,
        };

        (words, mask)
    }

    fn start_eater_once(self) {
        if EATER_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            thread::spawn(move || self.eater_loop());
        }
    }

    fn eater_loop(self) {
        thread::sleep(self.first_bite_offset());
        loop {
            thread::sleep(self.bite_offset());

            let len = ACTIVE_LEN.load(Ordering::Acquire);
            if len == 0 {
                continue;
            }

            let n = EVENTS.fetch_add(1, Ordering::Relaxed);
            let idx = n % len;
            let slot = ACTIVE[idx].load(Ordering::Acquire);
            if slot == EMPTY {
                continue;
            }

            let addr = REGISTRY[slot].addr.load(Ordering::Acquire);
            let size = REGISTRY[slot].size.load(Ordering::Relaxed);
            if addr == 0 || size < 64 {
                continue;
            }

            let (words, mask) = self.corruption_shape(n);
            if words == 0 || mask == 0 {
                continue;
            }

            let base = (size / 2) & !7;

            unsafe {
                for i in 0..words {
                    let off = base + i * 8;
                    if off + 8 > size {
                        break;
                    }
                    let p = (addr + off) as *mut u64;
                    let v = ptr::read(p);
                    ptr::write(p, v ^ mask);
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);

        if !ptr.is_null() && layout.size() >= 64 {
            if let Some(slot) = alloc_slot() {
                REGISTRY[slot].addr.store(ptr as usize, Ordering::Release);
                REGISTRY[slot].size.store(layout.size(), Ordering::Relaxed);
            }
        }

        self.start_eater_once();
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let addr = ptr as usize;
        let len = ACTIVE_LEN.load(Ordering::Acquire);

        // Bounded scan of dense ACTIVE set
        for i in 0..len {
            let slot = ACTIVE[i].load(Ordering::Acquire);
            if slot == EMPTY {
                continue;
            }

            if REGISTRY[slot].addr.load(Ordering::Acquire) == addr {
                free_slot(slot);

                // Compact ACTIVE by swap-remove
                let last = len - 1;
                let last_slot = ACTIVE[last].load(Ordering::Acquire);
                ACTIVE[i].store(last_slot, Ordering::Release);
                ACTIVE[last].store(EMPTY, Ordering::Release);
                ACTIVE_LEN.fetch_sub(1, Ordering::AcqRel);
                break;
            }
        }

        System.dealloc(ptr, layout)
    }
}

// === Activation ===

#[macro_export]
macro_rules! awaken {
    () => {
        $crate::awaken!(Hungry);
    };
    ($hunger:ident) => {
        #[global_allocator]
        static A: craturn::Allocator = craturn::Allocator {
            hunger: craturn::Hunger::$hunger,
        };
    };
}
