//! A cursed facade for using / sharing a RngCore implementation between components
//! using a global random instance and some RAII tricks.
//! 
//! ```
//! use std::pin::Pin;
//! use rand::{RngCore, SeedableRng};
//! use rand_chacha::ChaChaRng;
//! use rand_facade::GlobalRng;
//!
//! // Create new RNG instance (DO NOT USE A STATIC SEED IRL)
//! let mut chacha_rng = ChaChaRng::from_seed([1u8; 32]);
//! 
//! // Bind in to global RNG
//! let _rng_guard = GlobalRng::set(Pin::new(&mut chacha_rng));
//! 
//! // Use global RNG instances
//! let _rand = GlobalRng::get().next_u32();
//! 
//! ```


#![no_std]

use core::cell::RefCell;
use core::pin::Pin;
use core::marker::PhantomData;

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
    static ref GLOBAL_RNG: Mutex<RefCell<Option<&'static mut (dyn Rng + Sync + Send)>>> = Mutex::new(RefCell::new(None));
}

/// Rng trait requires both RngCore and CryptoRng
pub trait Rng: RngCore + CryptoRng {}

/// Auto impl for types already implementing RngCore and CryptoRng
impl <T> Rng for T where T: RngCore + CryptoRng {}


/// Wrapper providing mutex backed access to a global RNG instance
pub struct GlobalRng {}


/// Guard type holding the bound rng, when this is dropped the global 
/// RNG will become unavailable
pub struct RngGuard<'a> {
    rng: PhantomData<&'a (dyn Rng + Unpin)>,
}

impl <'a> Drop for RngGuard <'a> {
    fn drop(&mut self) {
        #[cfg(feature = "std")] {
            GLOBAL_RNG.lock().unwrap().replace(None);
        }
        
        #[cfg(feature = "cortex_m")]
        cortex_m::interrupt::free(move |cs| {
            GLOBAL_RNG.borrow(cs).replace(None)
        });
    }
}

impl GlobalRng {
    /// Fetch an instance of the global RNG
    pub fn get() -> Self {
        GlobalRng{}
    }

    /// Set the underlying instance for the global RNG
    /// 
    /// This extends the lifetime of the provided object to `static, and removes the
    /// global binding when the returned RngGuard is dropped.
    pub fn set<'a>(rng: Pin<&'a mut (dyn Rng + Unpin)>) -> RngGuard<'a> {
        // TODO: YIKES there's gotta be a better way
        let rng = unsafe { core::mem::transmute::<&'a mut (dyn Rng), &'static mut (dyn Rng + Sync + Send)>(rng.get_mut()) };
        
        #[cfg(feature = "std")] {
            GLOBAL_RNG.lock().unwrap().replace(Some(rng));
        }
        
        #[cfg(feature = "cortex_m")]
        cortex_m::interrupt::free(move |cs| {
            GLOBAL_RNG.borrow(cs).replace(Some(rng))
        });

        RngGuard{rng: PhantomData}
    }
}

/// GlobalRng instances must be CryptoRng
impl CryptoRng for GlobalRng {}


#[cfg(all(feature = "std", feature = "cortex_m"))]
compile_error!("Only one of 'std' or 'cortex_m' features may be enabled");

#[cfg(not(any(feature = "std", feature = "cortex_m")))]
compile_error!("One of 'std' or 'cortex_m' features must be enabled");


/// Standard library mutex based implementation
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


/// cortex-m mutex based implementation
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

#[cfg(test)]
mod test {

    use std::pin::Pin;
    use rand::{RngCore, SeedableRng};
    use rand_chacha::ChaChaRng;
    use super::GlobalRng;

    #[test]
    #[should_panic]
    fn drop_guard() {
        let mut chacha_rng = ChaChaRng::from_seed([1u8; 32]);
        
        let rng_guard = GlobalRng::set(Pin::new(&mut chacha_rng));

        drop(rng_guard);

        let _val = GlobalRng::get().next_u32();
    }
}
