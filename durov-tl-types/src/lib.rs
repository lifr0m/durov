pub mod serialize;
pub mod deserialize;
pub mod schemas;
pub mod buffer;
pub mod cursor;
mod utils;
mod constants;

use cursor::Cursor;
use deserialize::Deserialize;
use serialize::Serialize;
use std::any::Any;
use std::fmt::Debug;

pub trait Identify {
    const ID: i32;
}

pub trait Call {
    type Result: Deserialize;
}

#[derive(Debug, Clone)]
pub struct BareVec<T>(pub Vec<T>);

pub struct Object {
    pub id: i32,
    pub body: Box<dyn Any + Send>,
}

#[allow(clippy::type_complexity)]
pub fn multiple_deserialize_object(
    src: &mut Cursor,
    f_list: &[fn(&mut Cursor) -> Result<Object, deserialize::Error>],
) -> Result<Object, deserialize::Error> {
    Ok(match f_list[0](src) {
        Ok(object) => object,
        Err(deserialize::Error::UnknownId(_)) => {
            if f_list.len() <= 2 {
                f_list[1](src)?
            } else {
                multiple_deserialize_object(src, &f_list[1..])?
            }
        }
        Err(err) => return Err(err),
    })
}
