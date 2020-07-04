//! A cursed facade for using / sharing a RngCore implementation between components
//! using a global random instance and some RAII tricks.
//! 
//! ``` no_run
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
use core::marker::PhantomData;

use rand::{RngCore, CryptoRng, Error};
use lazy_static::lazy_static;

#[cfg(any(feature = "std", feature = "os_rng"))]
extern crate std;

#[cfg(any(feature = "std", feature = "os_rng"))]
use std::sync::Mutex;

#[cfg(feature = "cortex_m")]
use cortex_m::interrupt::Mutex;


#[cfg(all(feature = "std", feature = "cortex_m"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");

#[cfg(all(feature = "std", feature = "os_rng"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");

#[cfg(all(feature = "cortex_m", feature = "os_rng"))]
compile_error!("Only one of 'std', 'os_rng', or 'cortex_m' features may be enabled");


#[cfg(not(any(feature = "std", feature = "cortex_m", feature = "os_rng")))]
compile_error!("One of 'os_rng', 'std', 'cortex_m' features must be enabled");


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

/// GlobalRng instances must be CryptoRng
impl CryptoRng for GlobalRng {}


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
    /// Fetch an instance of the global RNG.
    /// 
    /// This can always be constructed, however, calling the RNG functions without
    /// having an appropriate RNG bound (or, defined by default with `os_rng`)
    /// will cause a panic.
    ///
    /// When `os_rng` is enabled this acts as a transparent wrapper over `rand::rngs::OsRng`.
    pub fn get() -> Self {
        return GlobalRng{};
    }

    /// Set the underlying instance for the global RNG
    /// 
    /// This extends the lifetime of the provided object to `static, and removes the
    /// global binding when the returned RngGuard is dropped.
    pub fn set<'a>(rng: core::pin::Pin<&'a mut (dyn Rng + Unpin)>) -> RngGuard<'a> {
        #[cfg(feature = "os_rng")]
        panic!("Global RNG binding is not available with `os_rng` feature");

        #[cfg(not(feature = "os_rng"))]
        {
            // Transmute from limited ('a) lifetime to `static
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
}


#[cfg(feature = "os_rng")]
impl rand::RngCore for GlobalRng {
    fn next_u32(&mut self) -> u32 {
        rand::rngs::OsRng.next_u32()
    }
    
    fn next_u64(&mut self) -> u64 {
        rand::rngs::OsRng.next_u64()
    }
    
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        rand::rngs::OsRng.fill_bytes(dest)
    }
    
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        rand::rngs::OsRng.try_fill_bytes(dest)
    }
}

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

#[cfg(all(test, feature="std"))]
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
