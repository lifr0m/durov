use crate::protocols::constants::GZIP_PACKED_ID;
use crate::protocols::encrypted::gzip::gzip_decode;
use crate::protocols::encrypted::object::{DeserializeObject, UnpackObject};
use durov_tl_types::cursor::{Cursor, Seek};
use durov_tl_types::deserialize;
use durov_tl_types::deserialize::Deserialize;

pub fn unpack_object(src: &mut Cursor, list: &[DeserializeObject])
    -> Result<UnpackObject, deserialize::Error>
{
    ungzip(src, &|src| {
        select_deserialize(src, list)
    })
}

fn ungzip(src: &mut Cursor, deserialize: DeserializeObject)
    -> Result<UnpackObject, deserialize::Error>
{
    let id = i32::deserialize(src)?;

    match id {
        GZIP_PACKED_ID => {
            let packed_data = Vec::<u8>::deserialize(src)?;
            let data = gzip_decode(&packed_data)?;
            let mut cur = Cursor::new(&data);
            deserialize(&mut cur)
        }
        _ => {
            src.seek(Seek::Backward(4));

            deserialize(src)
        }
    }
}

fn select_deserialize(src: &mut Cursor, list: &[DeserializeObject])
    -> Result<UnpackObject, deserialize::Error>
{
    match list[0](src) {
        Ok(object) => Ok(object),
        Err(deserialize::Error::IdMismatch { .. }) => {
            src.seek(Seek::Backward(4));

            if list.len() <= 2 {
                list[1](src)
            } else {
                select_deserialize(src, &list[1..])
            }
        }
        Err(err) => Err(err),
    }
}