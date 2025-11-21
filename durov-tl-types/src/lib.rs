pub mod serialize;
pub mod deserialize;
mod constants;
mod cursor;
mod utils;
pub mod schemas;

use deserialize::Deserialize;
use serialize::Serialize;

pub trait Identify {
    const ID: i32;
}

pub trait Call {
    type Result: Deserialize;
}

#[derive(Debug)]
pub struct BareVec<T>(pub Vec<T>);
