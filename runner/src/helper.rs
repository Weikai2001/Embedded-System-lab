use library_name::{Direction, Package, Command};

pub fn process_command(input_string: String) -> Option<Package> {
    // Remove any whitespace at the end
    let input_string = input_string.trim();

    // Split command and parameter (if needed)
    let mut input_parts = input_string.split_whitespace();
    let command = input_parts.next();
    let parameter = input_parts.next();
    if input_parts.next().is_some() {
        println!("Error: too many arguments");
    }

    match (command, parameter) {

        (Some("help"), None) => {
            print_help_list();
            return None
        }

        (Some("walk"), Some(dir_str)) => {
            if let Some(direction) = Direction::from_str(dir_str) {
                println!("Moving {:?}", direction);
                
                return Some(Package {
                    command: Command::walk,
                    parameter: Some(direction.to_u8()),
                })
            } else {
                println!("Invalid direction '{}'", dir_str);
                return None
            }
        }

        (Some("walk"), None) => {
            println!("Walk requires an additional direction parameter (N,E,S,W)");
            return None
        }

        (Some("step"), None) => {
            return Some(Package {
                command: Command::step,
                parameter: None,
            })
        }

        (Some("switch"), None) => {
            return Some(Package {
                command: Command::switch,
                parameter: None,
            })
        }

        (Some("reset"), None) => {
            return Some(Package {
                command: Command::reset,
                parameter: None,
            })
        }

        (Some("exit"), None) => {
            return Some(Package {
                command: Command::exit,
                parameter: None,
            })
        }

        (Some(other), None) => {
            println!("Unknown command: {}", other);
            return None
        }

        (Some(other), Some(p)) => {
            println!("Unknown command '{}' with parameter '{}'", other, p);
            return None
        }
        (None, _) => {
            println!("No command entered");
            return None
        }

    }

    fn print_help_list() {
        println!("Available commands:");
        println!("");
        println!("  To take a step:");
        println!("      walk *direction*, direction being either N,E,S,W");
        println!("");
        println!("  Request number of steps taken:");
        println!("      step");
        println!("");
        println!("  To switch view from map to step counter and other way around:");
        println!("      switch");
        println!("");
        println!("  To reset Stellaris board to initial state:");
        println!("      reset");
        println!("");
        println!("  To exit the program:");
        println!("      exit");
    }
}
