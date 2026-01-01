use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("missing bytes: data len {data_len}, pos {pos}, requested len {requested_len}")]
    MissingBytes {
        data_len: usize,
        pos: usize,
        requested_len: usize,
    },
}

pub enum Seek {
    Position(usize),
    Forward(usize),
    Backward(usize),
}

pub struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
        }
    }

    pub fn tell(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, at: Seek) {
        match at {
            Seek::Position(pos) => self.pos = pos,
            Seek::Forward(offset) => self.pos += offset,
            Seek::Backward(offset) => self.pos -= offset,
        }
    }

    pub fn read(&mut self, dst: &mut [u8]) -> Result<(), Error> {
        if self.pos + dst.len() > self.data.len() {
            return Err(Error::MissingBytes {
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
    fn test() {
        let data = [42, 50];
        let mut cur = Cursor::new(&data);

        let mut dst = [0; 1];
        cur.read(&mut dst).unwrap();
        assert_eq!(dst, [42]);
        assert_eq!(cur.tell(), 1);

        cur.seek(Seek::Backward(1));

        let mut dst = [0; 2];
        cur.read(&mut dst).unwrap();
        assert_eq!(dst, [42, 50]);

        let mut dst = [0; 1];
        cur.read(&mut dst).unwrap_err();
    }
}
