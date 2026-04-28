use crate::context::Context;
use crate::write::Write;
use crate::writer::Writer;
use durov_tl_parser::DataType;

impl Write for DataType {
    fn write(&self, writer: &mut Writer, context: &mut Context) {
        match self {
            DataType::Int => writer.raw_write("i32"),
            DataType::Long => writer.raw_write("i64"),
            DataType::Double => writer.raw_write("f64"),
            DataType::String => writer.raw_write("String"),
            DataType::Bytes => writer.raw_write("Vec::<u8>"),
            DataType::Bool => writer.raw_write("bool"),
            DataType::Vector(typ) => {
                writer.raw_write("Vec::<");
                typ.write(writer, context);
                writer.raw_write(">");
            }
            DataType::BareVector(typ) => {
                writer.raw_write("crate::BareVec::<");
                typ.write(writer, context);
                writer.raw_write(">");
            }
            DataType::Int128 => writer.raw_write("crypto_bigint::I128"),
            DataType::Int256 => writer.raw_write("crypto_bigint::I256"),
            DataType::Defined(name) => name.write(writer, context),
            DataType::Polymorphic(name) => writer.raw_write(name),
            DataType::PolymorphicFunction(name) => writer.raw_write(name),
            DataType::PolymorphicFunctionResult(name) => {
                writer.raw_write(name);
                writer.raw_write("::Result");
            }
            DataType::Condition => writer.raw_write("i32"),
            DataType::Conditional { typ, .. } => {
                writer.raw_write("Option::<");
                typ.write(writer, context);
                writer.raw_write(">");
            }
            DataType::ConditionalTrue { .. } => writer.raw_write("bool"),
            DataType::Boxed(typ) => {
                writer.raw_write("Box::<");
                typ.write(writer, context);
                writer.raw_write(">");
            }
        }
    }
}
