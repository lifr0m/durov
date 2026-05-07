use durov_tl_types::cursor::Cursor;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{deserialize, GetIdentifier};
use std::any::Any;

pub trait PackObjectTrait: Any + GetIdentifier + Serialize {}

impl<T: 'static + GetIdentifier + Serialize> PackObjectTrait for T {}

pub type PackObject = Box<dyn PackObjectTrait + Send>;

pub type UnpackObject = Box<dyn Any + Send>;

pub type DeserializeObject<'a> = &'a (
dyn Fn(&mut Cursor) -> Result<UnpackObject, deserialize::Error>
+ Sync
);

pub fn deserialize_object<T>(src: &mut Cursor) -> Result<UnpackObject, deserialize::Error>
where
    T: Deserialize + Send + 'static,
{
    let object = T::deserialize(src)?;
    Ok(Box::new(object))
}
