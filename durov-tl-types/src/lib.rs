pub mod serialize;
pub mod deserialize;
pub mod schemas;
pub mod buffer;
pub mod cursor;
mod constants;
mod utils;

pub trait Identify {
    const ID: i32;
}

pub trait Call {
    type Result;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BareVec<T>(pub Vec<T>);
