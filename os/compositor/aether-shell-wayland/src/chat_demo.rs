//! Deterministic token stream for the chat drawer smoke.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

const SAMPLE: &str = "# Streaming markdown\n\
This drawer parses tokens as they arrive. **Bold** and `inline code` keep their \
 styling across wraps.\n\
\n\
- first bullet\n\
- second bullet with **emphasis**\n\
\n\
```\n\
fn main() {\n\
    println!(\"streamed\");\n\
}\n\
```\n\
\n\
The gate cares about bounded work per frame, not token count. Each token only \
appends to the pending buffer; newline boundaries materialise a row.\n";

pub struct ChatDemo {
    rx: Receiver<String>,
}

impl ChatDemo {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut token = String::new();
            for character in SAMPLE.chars() {
                token.push(character);
                if character.is_whitespace() || character == '\n' {
                    if tx.send(std::mem::take(&mut token)).is_err() {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(12));
                }
            }
            if !token.is_empty() {
                let _ = tx.send(token);
            }
        });
        Self { rx }
    }

    /// Drain all pending tokens.
    pub fn drain(&self) -> Vec<String> {
        self.rx.try_iter().collect()
    }
}
