#![no_std]

use core::cell::RefCell;
use core::pin::Pin;

use rand::{RngCore, CryptoRng, Error};
use lazy_static::lazy_static;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::sync::Mutex;

#[cfg(feature = "cortex_m")]
use cortex_m::interrupt::Mutex;

lazy_static! {
    /// Global RNG instance
    static ref GLOBAL_RNG: Mutex<RefCell<Option<&'static mut (dyn RngCore + Sync + Send)>>> = Mutex::new(RefCell::new(None));
}

/// Wrapper providing mutex backed access to a global RNG instance
pub struct GlobalRng {}

impl GlobalRng {
    /// Fetch an instance of the global RNG
    pub fn get() -> Self {
        GlobalRng{}
    }

    /// Set the underlying instance for the global RNG
    /// This extends the lifetime of the provided object to `static so, better not go missing
    pub unsafe fn set<'a>(rng: Pin<&'a mut (dyn RngCore + Unpin)>) {
        // TODO: YIKES there's gotta be a better way
        let rng = core::mem::transmute::<&'a mut (dyn RngCore), &'static mut (dyn RngCore + Sync + Send)>(rng.get_mut());
        
        #[cfg(feature = "std")] {
            GLOBAL_RNG.lock().unwrap().replace(Some(rng));
        }
        
        #[cfg(feature = "cortex_m")]
        cortex_m::interrupt::free(move |cs| {
            GLOBAL_RNG.borrow(cs).replace(Some(rng))
        });
    }
}

impl CryptoRng for GlobalRng {}

#[cfg(all(feature = "std", feature = "cortex_m"))]
compile_error!("Only one of 'std' or 'cortex_m' features may be enabled");

#[cfg(not(any(feature = "std", feature = "cortex_m")))]
compile_error!("One of 'std' or 'cortex_m' features must be enabled");

#[cfg(feature = "std")]
impl rand::RngCore for GlobalRng {
    fn next_u32(&mut self) -> u32 {
        GLOBAL_RNG.lock().unwrap().borrow_mut().as_mut().unwrap().next_u32()
    }
    
    fn next_u64(&mut self) -> u64 {
        GLOBAL_RNG.lock().unwrap().borrow_mut().as_mut().unwrap().next_u64()
    }
    
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        GLOBAL_RNG.lock().unwrap().borrow_mut().as_mut().unwrap().fill_bytes(dest)
    }
    
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        GLOBAL_RNG.lock().unwrap().borrow_mut().as_mut().unwrap().try_fill_bytes(dest)
    }
}

#[cfg(feature = "cortex_m")]
impl rand::RngCore for GlobalRng {
    fn next_u32(&mut self) -> u32 {
        cortex_m::interrupt::free(|cs| {
            GLOBAL_RNG.borrow(cs).borrow_mut().as_mut().unwrap().next_u32()
        })
    }
    
    fn next_u64(&mut self) -> u64 {
        cortex_m::interrupt::free(|cs| {
            GLOBAL_RNG.borrow(cs).borrow_mut().as_mut().unwrap().next_u64()
        })
    }
    
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        cortex_m::interrupt::free(|cs| {
            GLOBAL_RNG.borrow(cs).borrow_mut().as_mut().unwrap().fill_bytes(dest)
        })
    }
    
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        cortex_m::interrupt::free(|cs| {
            GLOBAL_RNG.borrow(cs).borrow_mut().as_mut().unwrap().try_fill_bytes(dest)
        })
    }
}
