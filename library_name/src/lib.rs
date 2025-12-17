#![no_std]

use serde::{Serialize, Deserialize};
use postcard::{to_slice, from_bytes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North = 1,
    East = 2,
    South = 3,
    West = 4,
}

impl Direction {
    pub fn from_str(s: &str) -> Option<Self> {
        //match s.to_lowercase().as_str() {
        match s {
            "N" | "n" => Some(Direction::North),
            "E" | "e" => Some(Direction::East),
            "S" | "s" => Some(Direction::South),
            "W" | "w" => Some(Direction::West),
            _ => None,
        }
    }

    pub fn to_u8(&self) -> u8 {
        *self as u8
    }
} 

// Define a Command enum
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Command {
    walk,
    step,
    switch,
    reset,
    exit,
}

#[derive(Serialize, Deserialize)]
pub struct Package {
    pub command: Command,
    pub parameter: Option<u8>,
}

pub fn serialize_package<'a>(package: &Package, buffer: &'a mut[u8]) -> &'a[u8] {
    to_slice(package, buffer).unwrap()
} 

pub fn deserialize_package(raw_bytes: &[u8]) -> Option<Package> {
    match from_bytes::<Package>(raw_bytes) {
        Ok(package) => Some(package),
        Err(_) => None,
    }
}
