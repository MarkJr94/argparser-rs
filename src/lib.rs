#![warn(missing_docs)]

pub mod argparser;
pub mod slide;

pub use argparser::{ArgParser, ArgType, hashmap_parser, vec_parser};