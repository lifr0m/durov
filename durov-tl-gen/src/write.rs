use crate::context::Context;
use crate::writer::Writer;

pub trait Write {
    fn write(&self, writer: &mut Writer, context: &mut Context);
}

impl<T: Write> Write for &T {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        T::write(self, writer, context);
    }
}
