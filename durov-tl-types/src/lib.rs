mod serialize;
mod deserialize;
mod constants;
mod cursor;
mod utils;
pub mod schemas;

pub use deserialize::Deserialize;
pub use serialize::Serialize;

pub trait Identify {
    const ID: i32;
}

pub trait Call {
    type Result: Deserialize;
}

pub struct BareVec<T>(pub Vec<T>);
