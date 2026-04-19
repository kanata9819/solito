pub struct InputBuffer {
    buffer: Vec<char>,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn pop(&mut self) -> Option<char> {
        self.buffer.pop()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn get_string(&self) -> String {
        self.buffer.iter().collect::<String>()
    }
}
