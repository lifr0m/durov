use durov_mtproto::protocols::encrypted::object::{DeserializeObject, PackObject, UnpackObject};

pub enum Request {
    Service(PackObject),
    Rpc(CallData),
}

pub struct CallData {
    pub body: PackObject,
    pub callback: flume::Sender<UnpackObject>,
    pub deserialize: DeserializeObject<'static>,
}
