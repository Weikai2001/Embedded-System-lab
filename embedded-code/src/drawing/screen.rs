use crate::drawing::brightness::Brightness;
use crate::drawing::font::{Character, NUMBERS};
use tudelft_lm3s6965_pac::{GPIO_PORTC, SSI0};
use crate::Vec;

pub struct Screen<'p> {
    ssi: &'p mut SSI0,
    gpio: &'p mut GPIO_PORTC,
    pub fb: [[u8; (Screen::WIDTH / 2) as usize]; Screen::HEIGHT as usize],
}

impl<'p> Screen<'p> {
    pub const WIDTH: u8 = 128;
    pub const HEIGHT: u8 = 80;

    pub fn new(ssi: &'p mut SSI0, gpio: &'p mut GPIO_PORTC) -> Self {
        // 1. Ensure that the SSE bit in the SSICR1 register is disabled before making any configuration changes.
        ssi.cr1.write(|w| w.ssi_cr1_sse().clear_bit());

        // 2. Select whether the SSI is a master or slave:
        //     a. For master operations, set the SSICR1 register to 0x0000.0000.
        //     b. For slave mode (output enabled), set the SSICR1 register to 0x0000.0004.
        //     c. For slave mode (output disabled), set the SSICR1 register to 0x0000.000C.
        ssi.cr1.write(|w| w.ssi_cr1_ms().clear_bit());

        // 3. Configure the clock prescale divisor by writing the SSICPSR register.
        // SAFETY: according to the docs, 2 is a valid value for this register
        ssi.cpsr.write(|w| unsafe { w.ssi_cpsr_cpsdvsr().bits(2) });

        // 4. Write the SSICR0 register with the following configuration:
        //     ■ Serial clock rate (SCR)
        //     ■ Desired clock phase/polarity, if using Freescale SPI mode (SPH and SPO)
        //     ■ The protocol mode: Freescale SPI, TI SSF, MICROWIRE (FRF)
        //     ■ The data size (DSS)
        // SAFETY: according to the docs, 9 is a valid value for this register
        ssi.cr0.write(|w| unsafe { w.ssi_cr0_scr().bits(9) });

        // 5. Enable the SSI by setting the SSE bit in the SSICR1 register.
        ssi.cr1.write(|w| w.ssi_cr1_sse().set_bit());

        // 6. set the bitmask
        ssi.cr0.write(|w| w.ssi_cr0_dss().ssi_cr0_dss_16());

        // SAFETY: according to the docs, these are both valid values for these two registers
        gpio.den.write(|w| unsafe { w.bits(1) });
        gpio.dir.write(|w| unsafe { w.bits(0xff) });

        Self {
            ssi,
            gpio,
            fb: [[0; (Self::WIDTH / 2) as usize]; Self::HEIGHT as usize],
        }
    }

    fn write_ssi(&mut self, data: u16) {
        self.ssi.dr.write(|w| unsafe { w.ssi_dr_data().bits(data) });
        let _ = self.ssi.dr.read();
        while self.ssi.sr.read().ssi_sr_bsy().bit_is_set() {}
    }

    fn change_mode(&mut self, mode: Mode) {
        // SAFETY: these two values registers can have any 7 bit value.
        // the exact values correspond with the ones qemu expects here
        // which we checked by reading qemu's source code.
        match mode {
            Mode::Cmd => self.gpio.data.write(|w| unsafe { w.bits(0x00) }),
            Mode::Data => self.gpio.data.write(|w| unsafe { w.bits(0xa0) }),
        }
    }

    /// Assumes we are in command mode and min/max are in bounds
    fn set_col(&mut self, min_curr: u8, max: u8) {
        self.write_ssi(0x15);
        self.write_ssi(min_curr as u16);
        self.write_ssi(max as u16);
    }

    /// Assumes we are in command mode and min/max are in bounds
    fn set_row(&mut self, min_curr: u8, max: u8) {
        self.write_ssi(0x75);
        self.write_ssi(min_curr as u16);
        self.write_ssi(max as u16);
    }

    pub fn draw_pixel(&mut self, x: u8, y: u8, brightness: Brightness) {
        assert!(x < 128, "x larger than width");
        assert!(y < 64, "y larger than height");

        self.change_mode(Mode::Cmd);
        self.set_col(x / 2, Self::WIDTH - 1);
        self.set_row(y, Self::HEIGHT - 1);

        self.change_mode(Mode::Data);

        let current = &mut self.fb[y as usize][x as usize / 2];
        if x % 2 == 1 {
            *current &= 0xf0;
            *current |= Into::<u8>::into(brightness);
        } else {
            *current &= 0x0f;
            *current |= Into::<u8>::into(brightness) << 4;
        }

        let value = *current;
        self.write_ssi(value as u16);
    }

    pub fn clear(&mut self, brightness: Brightness) {
        self.change_mode(Mode::Cmd);
        self.set_row(0, Self::HEIGHT - 1);
        self.set_col(0, Self::WIDTH - 1);

        self.change_mode(Mode::Data);

        let brightness: u8 = brightness.into();
        let pix = brightness | (brightness << 4);

        for x in 0..(Self::WIDTH / 2) as usize {
            for y in 0..Self::HEIGHT as usize {
                self.write_ssi(pix as u16);
                self.fb[y][x] = pix;
            }
        }
    }

    /// Draw a character at a given position
    /// x: 0..127
    /// y: 0..79
    pub fn draw_char(&mut self, x: u8, y: u8, char: &Character, brightness: Brightness){

        for (row_idx, row) in char.iter().enumerate(){
            for (col_idx, &pixel) in row.iter().enumerate(){
                if pixel {
                    self.draw_pixel(x + col_idx as u8, y + row_idx as u8, brightness);
                }
            }
        }
    }

    /// Draw number at a given position
    pub fn draw_number(&mut self, x: u8, y: u8, number: u32, brightness: Brightness){
        // Convert number to digits 
        let digits: Vec<u8,10> = {
            let mut v: Vec<u8, 10> = Vec::new();
            let mut n = number;
            if n == 0 {
                v.push(0).unwrap();
            } else {
                let mut rev_digits: Vec<u8, 10> = Vec::new();
                while n > 0 {
                    rev_digits.push((n % 10) as u8).unwrap();
                    n /= 10;
                }
                
                // Reverse the digits to get the correct order
                for &d in rev_digits.iter().rev(){
                    v.push(d).unwrap();
                }
            }
        v
        };
    
        // Draw each digit
        let mut x_offset = 0;
        for &digit in digits.iter(){
            let char_ref: &Character = &NUMBERS[digit as usize];
            self.draw_char(x + x_offset, y, char_ref, brightness);
            x_offset += 12; // each char is 8 pixels wide
        }
    }
        
 
    /// Redraw the entire framebuffer to the screen without clear it
    pub fn redraw_fb(&mut self){
        self.change_mode(Mode::Cmd);
        self.set_row(0, Self::HEIGHT - 1);
        self.set_col(0, Self::WIDTH -1);
        self.change_mode(Mode::Data);

        for y in 0..Self::HEIGHT as usize {
            for x in 0..(Self:: WIDTH /2) as usize {
                let value = self.fb[y][x];
                self.write_ssi(value as u16);
            }
        }

    }
    
}


enum Mode {
    Cmd,
    Data,
}
