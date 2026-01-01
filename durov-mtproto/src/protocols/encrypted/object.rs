use durov_tl_types::cursor::Cursor;
use durov_tl_types::deserialize::Deserialize;
use durov_tl_types::serialize::Serialize;
use durov_tl_types::{deserialize, Identify};
use std::any::Any;
use std::sync::Arc;

pub type Object = Box<dyn Any + Send>;
pub type DeserializeObject = fn(&mut Cursor) -> Result<Object, deserialize::Error>;

pub struct InObject {
    pub id: i32,
    pub body: Arc<dyn Serialize + Send + Sync>,
}

impl InObject {
    pub fn new<T>(body: T) -> Self
    where
        T: Identify + Serialize + Send + Sync + 'static,
    {
        Self {
            id: T::ID,
            body: Arc::new(body),
        }
    }
}

pub struct OutObject {
    pub msg_id: i64,
    pub body: Object,
}

impl OutObject {
    pub fn new(msg_id: i64, body: Object) -> Self {
        Self { msg_id, body }
    }
}

pub fn deserialize_object<T>(src: &mut Cursor) -> Result<Object, deserialize::Error>
where
    T: Deserialize + Send + 'static,
{
    let object = T::deserialize(src)?;
    Ok(Box::new(object))
}
