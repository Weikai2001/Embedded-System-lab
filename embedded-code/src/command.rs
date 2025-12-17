use crate::uart::Uart;
use crate::drawing::screen::Screen;
use crate::drawing::brightness::Brightness;
use library_name::{Package, Command};
use core::fmt::Write;

/// This state will keep track the location, steps, views and log the path
pub struct State {
    pub x: u8,
    pub y: u8,
    pub steps: u32,
    pub map_view: bool,
    pub visited: heapless::Vec<(u8, u8), 1024> // 1024 Pixels of storage
}

impl State {
    pub fn new() -> Self {
        Self {
            x: Screen::WIDTH /2,
            y: Screen::HEIGHT /2,
            steps: 0,
            map_view: true,  
            visited: heapless::Vec::new(),
        }
    }
}

/// This function handles all received packages
/// Packages includes two Things: 
/// Command and parameter(N, S, W, E)
pub fn handle_package(package: &Package, state: &mut State, screen: &mut Screen, uart: &mut Uart){
    match package.command {
        Command::walk => {
            // package.parameter is Option<u8>
            match package.parameter {
                Some(1) => walk(0, -1, state, screen, uart),  // S
                Some(2) => walk(1, 0, state, screen, uart), // E
                Some(3) => walk(0, 1, state, screen, uart),  // N
                Some(4) => walk(-1, 0, state, screen, uart), // W
                _ => {} // ignore missing or unknown parameter
            }
        },

        Command::reset => reset(state, screen, uart),
        Command::switch => switch_view(state, screen, uart),
        Command::step => request_steps(state, uart),

        // Exit already handled at the in the runner

        _ => {} // ignore unknown commands 

    }
}

/// Update the screen and state 
pub fn walk(dx: i8, dy: i8, state: &mut State, screen: &mut Screen, uart: &mut Uart){
    state.x = (state.x as i16 + dx as i16).clamp(0, (Screen::WIDTH -1) as i16) as u8; 
    state.y = (state.y as i16 + dy as i16).clamp(0, (Screen::HEIGHT -1) as i16) as u8;
    state.steps += 1;

    // Log the current position
    let _ = state.visited.push((state.x, state.y));


    // Normal map view
    if state.map_view {
        screen.draw_pixel(state.x, state.y, Brightness::PATH);
    }
    // Step view
    else {
        screen.clear(Brightness::WHITE);
        screen.draw_number( 40, 20, state.steps, Brightness::BLACK);
    }
    writeln!(uart, "OK walked a step").ok();
}


/// Reset the state and screen
pub fn reset(state: &mut State, screen: &mut Screen, uart: &mut Uart){
    state.x = Screen::WIDTH / 2;
    state.y = Screen::HEIGHT / 2;
    state.steps = 0;
    screen.clear(Brightness::WHITE);
    state.visited.clear();
    state.visited.push((state.x, state.y)).unwrap();

    
    if !state.map_view{
        screen.draw_number(40, 20, state.steps, Brightness::BLACK);
    } else {
        //Draw the starting pixel
        screen.draw_pixel(state.x, state.y, Brightness::BLACK);
    }
    writeln!(uart, "OK reset screen").ok();
}

/// Switch between map view and step view
pub fn switch_view(state: &mut State, screen: &mut Screen, uart: &mut Uart){
    state.map_view = !state.map_view;

    if state.map_view {
        screen.clear(Brightness::WHITE);
        // Redraw all logged pixels 
        for (i, &(x, y)) in state.visited.iter().enumerate(){
            if i == 0 {
              screen.draw_pixel(x, y, Brightness::BLACK);
            } else {
              screen.draw_pixel(x, y, Brightness::PATH);
            }
        }
        writeln!(uart, "OK switched to map view").ok();
    } else {
        screen.clear(Brightness::WHITE);
        screen.draw_number(40, 20, state.steps, Brightness::BLACK);
        writeln!(uart, "OK switched to number of steps view").ok();
    }
}

/// Request the steps to the Stellaris board
pub fn request_steps(state: &State, uart: &mut Uart){
    writeln!(uart, "OK steps taken: {}", state.steps).ok();
}

