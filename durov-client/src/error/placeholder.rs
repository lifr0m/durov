pub trait Placeholder {
    const PLACEHOLDER: &str;
}

impl Placeholder for i32 {
    const PLACEHOLDER: &str = "%d";
}
