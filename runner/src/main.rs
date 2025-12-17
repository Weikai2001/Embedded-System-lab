use std::env::args;
use std::io::{stdout, stdin, Read, Write};

use tudelft_arm_qemu_runner::Runner;

mod helper;
use helper::process_command;
use library_name::{Command, serialize_package};

const START_BYTE: u8 = 0xAA;
const END_BYTE: u8 = 0x55;


/// CRC-8 (polynomail 0x07)
fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;

    for &byte in data {
        crc ^= byte; // XOR operation
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

fn main() -> color_eyre::Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let binary = args().nth(1).unwrap();
    let mut runner: Runner = Runner::new(&binary, false)?;

    let mut write_buf = [0u8; 32];
    println!("Type 'help' to see available commands");

    // Delete this after testing 
    let mut corrupt_next = true;

    loop {
        print!("> ");
        stdout().lock().flush().unwrap();

        // Get command from user
        let mut input_string = String::new();
        stdin().read_line(&mut input_string).unwrap();
        
        // If input is empty, skip
        if input_string.is_empty() {
            continue;
        }
        

        // Check which command is input and act accordingly
        match process_command(input_string) {
            Some(package) => {
                // If the exit command given we quit the program
                if package.command == Command::exit {
                    println!("Exiting...");
                    break Ok(())
                }
                
                // Convert struct to bytes to be able to transmit over uart
                let payload = serialize_package(&package, &mut write_buf);

                // lengte of the package bytes 
                let len = payload.len() as u8;


                // Build CRC input: LEN + PAYLOAD
                let mut crc_input = Vec::with_capacity(1+payload.len());
                crc_input.push(len);
                crc_input.extend_from_slice(&payload);

                // Compute CRC-8
                let crc = crc8(&crc_input);

                // /// Delete this after testing
                // // Prepare the payload to send
                // let mut payload = payload.to_vec();

                // // Delete this after testing
                // // // --- ONE-TIME CORRUPTION ---
                // // if corrupt_next {
                // //     payload[1] = 0;   // Flip/zero the byte
                // //     corrupt_next = false; // Only corrupt this one time
                // // }

                // // Delete this after testing
                // // --- One time byte drop ---
                // if corrupt_next {
                //     let dropped = payload.remove(1);
                //     corrupt_next = false;
                // }

                // Send the bytes of START, LEN, PAYLOAD, CRC and END to the MCU
                runner.stream.write_all(&[START_BYTE])?;
                runner.stream.write_all(&[len])?;
                runner.stream.write_all(&payload)?;
                runner.stream.write_all(&[crc])?;
                runner.stream.write_all(&[END_BYTE])?;
    
            }
            None => {
                continue;
            }
        }

        // Print the return message from the microcontroller
        let mut temp_string = String::new();
        let mut read_buf = [0u8; 64];
        loop {
            // Read from the uart stream
            let n = runner.stream.read(&mut read_buf)?;
            if n == 0 {
                continue;
            }

            let chunk = String::from_utf8_lossy(&read_buf[..n]);
            temp_string.push_str(&chunk);

            // Check if we received a full line (we assume return messages end with '\n')
            if let Some(pos) = temp_string.find('\n') {
                let line = temp_string[..pos].trim().to_string();
                temp_string = temp_string[pos + 1..].to_string();

                println!("Return message: {}", line);

                break;
            }
        }
    }
}
