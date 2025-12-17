use core::fmt::Write;
use core::mem::MaybeUninit;
use tudelft_lm3s6965_pac::{interrupt,UART0,SYSCTL,GPIO_PORTA};

use crate::mutex::Mutex;

/// Fixed size buffer for UART RX data
pub struct UartBuffer {
    buffer: [u8; 128],
    head: usize,
    tail: usize,
}

impl UartBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: [0; 128],
            head: 0,
            tail: 0,
        }
    }

    /// Push a byte into the buffer
    pub fn push(&mut self, byte: u8) -> Result<(), ()> {
        let next = (self.head + 1) % self.buffer.len();
        if next == self.tail {
            // Buffer is full - cannot push new data now
            return Err(());
        }
        self.buffer[self.head] =byte;
        self.head = next;
        Ok(())
    }

    /// Pop a byte from the buffer. Return byte or None if empty
    pub fn pop(&mut self) -> Option<u8> {
        if self.head == self.tail {
            return None;
        }
        let byte = self.buffer[self.tail];
        self.tail = (self.tail + 1) % self.buffer.len();
        Some(byte)
    }
}

pub struct Uart {
    uart: UART0,
    pub rx_buffer: Mutex<UartBuffer>,
}

impl Uart {
    pub fn new(uart: UART0, sysctl: SYSCTL, gpio: GPIO_PORTA) -> Self {
        
        // CLOCK settings:
        sysctl.rcgc1.modify(|_, w| w.sysctl_rcgc1_uart0().set_bit()); // Enable clock for UART0
        sysctl.rcgc2.modify(|_, w| w.sysctl_rcgc2_gpioa().set_bit()); // Enable clock for GPIOA
        
        // GPIO settings:
        // In GPIO there is no access to safe accessor methods so we need to use unsafe to manually
        // set the bits of the register fields
        gpio.afsel.write(|w| unsafe { w.bits(0b11) }); // Enable PA0/PA1 pins alternate function
        gpio.den.write(|w| unsafe { w.bits(0b11) }); // Make PA0/PA1 digital
        gpio.dir.write(|w| unsafe { w.bits(0b10) }); // Sets PA0 to input (tx) and PA1 to output (tx)

        // UART settings:
        uart.ctl.modify(|_, w| w.uart_ctl_uarten().clear_bit()); // Disable UART
        
        // Set baudrate to 115200baud (for 8MHz), using unsafe as bits() is unsafe for writing an arbitrary
        // number as there is no guarantee the number won't create problems (overflows and such)
        uart.ibrd.write(|w| unsafe { w.uart_ibrd_divint().bits(4) }); 
        uart.fbrd.write(|w| unsafe { w.uart_fbrd_divfrac().bits(22) });
        uart.lcrh.modify(|_, w| { 
            w.uart_lcrh_wlen().bits(0b11) // Set to 8 databits per message
            .uart_lcrh_fen().set_bit() // Enable FIFO
        }); 
        uart.ctl.modify(|_, w| {
            w.uart_ctl_uarten().set_bit() // Enable UART
            .uart_ctl_txe().set_bit() // Enable tx
            .uart_ctl_rxe().set_bit() // Enable rx
        });

        // Enable RX interrupts
        uart.im.write(|w| w.uart_im_rxim().set_bit());

        Self {
            uart,
            rx_buffer: Mutex::new(UartBuffer::new()),
        }
    }

    pub fn write(&mut self, value: &[u8]) {
        for &b in value {
            // Wait until transfer FIFO is not full 
            while self.uart.fr.read().uart_fr_txff().bit_is_set() {}
            
            // Write to UART data register
            self.uart.dr.write(|w| unsafe{ w.uart_dr_data().bits(b) });
        } 
    }

    // Read a byte from RX buffer
    pub fn read(&mut self) -> Option<u8> {
        // Use mutex to acces rx buffer
        self.rx_buffer.update(|b| b.pop())
    }

}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}

///UART0 interrupt handler 
/// This function is called when UART0 receives new data
#[interrupt]
unsafe fn UART0() {
    let uart = &*UART0::ptr();

    // Pointer to global uart driver
    let uart_global = &mut *UART0_INSTANCE.as_mut_ptr();
    
    // Read all received bytes and push them to rx buffer
    while uart.fr.read().uart_fr_rxfe().bit_is_clear() {
        let byte = uart.dr.read().uart_dr_data().bits();

        // Push byte to rx buffer using mutex
        let _ = uart_global.rx_buffer.update(|b| b.push(byte));
    }

    // Clear interrupt flag (reset Rx interrupt)
    uart.icr.write(|w| w.uart_icr_rxic().set_bit());
}

/// Global UART0 instance
#[no_mangle]
pub static mut UART0_INSTANCE: MaybeUninit<Uart> = MaybeUninit::uninit();
