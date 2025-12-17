#![no_std]
#![no_main]

extern crate cortex_m_rt as rt;
extern crate tudelft_lm3s6965_pac as _;

use crate::uart::{UART0_INSTANCE, Uart};
use core::arch::asm;
use cortex_m_semihosting::hprint;
use drawing::brightness::Brightness;
use drawing::screen::Screen;
use rt::entry;
use heapless::Vec;
use tudelft_lm3s6965_pac::{Peripherals, UART0, NVIC, Interrupt};
use library_name::{Package, deserialize_package};
use command::{State, handle_package};
use core::fmt::Write;

mod drawing;
mod exceptions;
mod uart;
mod command;

mod mutex;

const START_BYTE: u8 = 0xAA;
const END_BYTE: u8 = 0x55;
const MAX_PAYLOAD: usize = 32;

/// CRC-8 (polynomail 0x07),
/// Used for missing byte and corrupted byte check
fn crc8_update(mut crc: u8, byte: u8) -> u8 {
    crc ^= byte; // XOR operation
    for _ in 0..8 {
        if (crc & 0x80) != 0 {
            crc = (crc << 1) ^ 0x07;
        } else {
            crc <<= 1;
        }
    }
    crc
}

enum RxState{
    WaitStart,
    WaitLen,
    ReadPayload,
    ReadCrc,
    WaitEnd,
}


#[entry]
fn main() -> ! {
    let mut dp = Peripherals::take().unwrap();

    // initialize the screen for drawing
    let mut screen = Screen::new(&mut dp.SSI0, &mut dp.GPIO_PORTC);
    screen.clear(Brightness::WHITE);

    // Create initial state
    let mut state = State::new();

    // Draw initial position at the middle
    screen.draw_pixel(state.x, state.y, Brightness::BLACK);

    // Unsafe because we directly access and initialize  UART0 and NVIC
    // Ensure safety calling only once during startup
    unsafe{
        UART0_INSTANCE.write(Uart::new(dp.UART0, dp.SYSCTL, dp.GPIO_PORTA));
        NVIC::unmask(Interrupt::UART0);
    }
    
    // Enable UART0 interrupt
    let uart_hw = unsafe { &*UART0::ptr()};
    uart_hw.im.write(|w| w.uart_im_rxim().set_bit());
    let uart = unsafe {&mut *UART0_INSTANCE.as_mut_ptr()};

    // Variables for checksum state machine
    let mut rx_state =RxState::WaitStart;
    let mut payload_len = 0usize;
    let mut payload_buf = [0u8; MAX_PAYLOAD];
    let mut payload_pos: usize = 0;
    let mut received_crc = 0u8;
    let mut crc_calc: u8 = 0;


    loop {

        // Loop through all received bytes and push them through the state machine
        while let Some(byte) = uart.read() {


            // RX state machine with 5 steps:
            // 1) WaitStart: Look for the start signal at the beginning of a new message, 0xAA
            // 2) WaitLen: Read the payload length to know how many bytes to expect for payload
            // 3) ReadPayLoad: Read the payload bytes into a buffer
            // 4) ReadCrc: Read the crc value, check if the end value is already taken
            // 5) WaitEnd: Wait for the end signal, 0x55
            match rx_state {


                RxState::WaitStart => {
                    if byte == START_BYTE {
                        payload_pos = 0;
                        rx_state = RxState::WaitLen;      
                    }
                }


                RxState::WaitLen => {
                    if byte <= MAX_PAYLOAD as u8 {
                        payload_len = byte as usize;
                        payload_pos = 0;

                        // Push the length into crc
                        crc_calc = crc8_update(0, byte);
                        rx_state = RxState::ReadPayload; 
                    } else {
                        // Invalid length
                        rx_state = RxState::WaitStart;
                    }
                }


                RxState::ReadPayload => {
                    if byte == START_BYTE {
                        rx_state = RxState::WaitLen;
                        payload_pos = 0;
                        crc_calc = 0;
                        continue;
                    }

                    // Push in the payload and +1 the index in each iteration
                    payload_buf [payload_pos] = byte;
                    payload_pos += 1; 

                    // Push the previous crc_calc with the payload
                    crc_calc = crc8_update(crc_calc, byte);
         

                    if payload_pos == payload_len {
                            rx_state = RxState::ReadCrc;
                    }
                }
                
                

                RxState::ReadCrc => {
                    if byte == START_BYTE {
                        rx_state = RxState::WaitLen;
                        payload_pos = 0;
                        crc_calc = 0;
                        continue;
                    }

                    received_crc = byte;
                    rx_state = RxState::WaitEnd;

                    // If this crc equals to then end value, means one byte lose
                    // Reset and notice this failure
                    if  received_crc == 0x55 {
                        rx_state = RxState::WaitStart;
                        payload_pos = 0;
                        crc_calc = 0;
                        payload_len = 0;
                        writeln!(uart," CRC fail, missing a byte in payload").ok();
                    }
                }


                RxState::WaitEnd => {
                    if byte == START_BYTE {
                        rx_state = RxState::WaitLen;
                        payload_pos = 0;
                        continue;
                    }

                    if byte == END_BYTE {         
                        // Normal situation - frame is received successfully 
                        if crc_calc == received_crc {
                            if let Some(package) = deserialize_package(&payload_buf[..payload_len]){
                                handle_package(&package, &mut state, &mut screen, uart);
                            }
                        } else {
                            // Bad situation - corrupted byte, do not pass the crc test
                            writeln!(uart, "CRC FAIL due to corrupted byte").ok();
                            }
                    }
                
                // Go back to the start state and reset everything
                rx_state = RxState::WaitStart;
                payload_pos = 0;
                crc_calc = 0;
                payload_len = 0;
                }    

            }
        }

        // wait for interrupts, before looping again to save cycles.
        unsafe { asm!("wfi") }
    }
}
