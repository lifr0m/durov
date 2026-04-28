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

pub trait GetIdentifier {
    fn id(&self) -> i32;
}

impl<T: Identify> GetIdentifier for T {
    fn id(&self) -> i32 {
        T::ID
    }
}

pub trait Call {
    type Result;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BareVec<T>(pub Vec<T>);
