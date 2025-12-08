pub mod serialize;
pub mod deserialize;
pub mod schemas;
pub mod buffer;
pub mod cursor;
mod constants;
mod utils;

use deserialize::Deserialize;
use serialize::Serialize;
use std::fmt::Debug;

// todo
pub trait Identify {
    const ID: i32;
}

// todo
pub trait Call {
    type Result: Deserialize;
}

#[derive(Debug, Clone)]
pub struct BareVec<T>(pub Vec<T>);
