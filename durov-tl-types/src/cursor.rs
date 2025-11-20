use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("data is too short: data len {data_len}, pos {pos}, requested len {requested_len}")]
    DataTooShort {
        data_len: usize,
        pos: usize,
        requested_len: usize,
    },
}

pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn consumed(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.consumed()
    }

    pub fn read(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        if self.pos + dst.len() > self.data.len() {
            return Err(Error::DataTooShort {
                data_len: self.data.len(),
                pos: self.pos,
                requested_len: dst.len(),
            });
        }

        dst.copy_from_slice(&self.data[self.pos..self.pos + dst.len()]);
        self.pos += dst.len();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read() {
        let data = vec![42, 50];
        let mut cur = Cursor::new(&data);

        let mut dst = [0; 1];
        cur.read(&mut dst)
            .unwrap();
        assert_eq!(dst, [42]);

        let mut dst = [0; 1];
        cur.read(&mut dst)
            .unwrap();
        assert_eq!(dst, [50]);

        let mut dst = [0; 1];
        cur.read(&mut dst)
            .unwrap_err();
    }
}
