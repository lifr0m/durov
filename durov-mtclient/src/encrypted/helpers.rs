use std::iter;
use std::marker::PhantomData;

pub trait Chunks: Iterator {
    fn chunks<C>(self, chunk_size: usize) -> ChunksIter<Self, C>
    where
        Self: Sized,
    {
        ChunksIter {
            iter: self,
            chunk_size,
            _collection: PhantomData,
        }
    }
}

impl<T: Iterator> Chunks for T {}

pub struct ChunksIter<I, C> {
    iter: I,
    chunk_size: usize,
    _collection: PhantomData<C>,
}

impl<I: Iterator, C: FromIterator<I::Item>> Iterator for ChunksIter<I, C> {
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
            .map(|item| {
                let iter = iter::once(item)
                    .chain(&mut self.iter)
                    .take(self.chunk_size);
                C::from_iter(iter)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunks() {
        let vec = vec![1, 2, 3];
        let mut iter = vec.into_iter().chunks(2);
        assert_eq!(iter.next(), Some(vec![1, 2]));
        assert_eq!(iter.next(), Some(vec![3]));
        assert_eq!(iter.next(), None);
    }
}
