pub struct Writer {
    code: String,
    indent: usize,
    indent_step: usize,
}

impl Writer {
    pub fn new(indent: usize) -> Self {
        Self {
            code: String::new(),
            indent: 0,
            indent_step: indent,
        }
    }

    pub fn raw_write(&mut self, code: &str) {
        self.code.push_str(code);
    }

    pub fn indent_write(&mut self, code: &str) {
        for _ in 0..self.indent {
            self.code.push(' ');
        }
        self.raw_write(code);
    }

    pub fn code_write(&mut self, code: &str) {
        for line in code.split("\n") {
            self.indent_write(line);
            self.raw_write("\n");
        }
    }

    pub fn add_indent(&mut self) {
        self.indent += self.indent_step;
    }

    pub fn subtract_indent(&mut self) {
        self.indent -= self.indent_step;
    }

    pub fn destruct(self) -> String {
        self.code
    }
}
