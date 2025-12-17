use cortex_m::interrupt;
use core::cell::UnsafeCell;

// A simple mutex 
pub struct Mutex<T> {
    data: core::cell::UnsafeCell<T>,
}

// Safety: ensure that T is only accessed in critical sections
unsafe impl<T> Sync for Mutex<T> {}


impl<T> Mutex<T> {
    // make a new mutex
    pub fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }

    // "locks" the mutex, and runs v as a critical section
    // Interrupt are disabled during the execution
    pub fn update<R>(&self, v: impl FnOnce(&mut T) -> R) -> R {

        // run the closure in a critical section
        interrupt::free(|_| {
            // Safety: interrupts are disabled, no other code can access data 
            let result = v(unsafe { &mut *self.data.get()});
            result
        })
    }
}
