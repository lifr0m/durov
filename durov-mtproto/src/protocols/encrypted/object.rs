use durov_tl_types::serialize::Serialize;
use durov_tl_types::{Identify, Object};

pub struct InObject {
    pub id: i32,
    pub body: Box<dyn Serialize + Send>,
}

impl InObject {
    pub fn new<T>(body: T) -> Self
    where
        T: Identify + Serialize + Send + 'static,
    {
        Self {
            id: T::ID,
            body: Box::new(body),
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
