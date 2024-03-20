use core::fmt;
use std::io;

#[derive(Debug)]
pub struct LogStream {
    logs: Vec<String>,
    current_bytesize: usize,
}

impl Default for LogStream {
    fn default() -> Self {
        let logs = Vec::new();
        let current_bytesize = 0;
        Self {
            logs,
            current_bytesize,
        }
    }
}

impl fmt::Display for LogStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for message in &self.logs {
            write!(f, "{message}")?;
        }
        Ok(())
    }
}

impl io::Write for LogStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(self.append(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogStream {
    /// Append a buffer to the log stream.
    ///
    /// # Arguments
    /// * `buf` - the buffer to append
    pub fn append(&mut self, buf: &[u8]) -> usize {
        let log = String::from_utf8_lossy(buf);

        let log_length = log.len();
        self.current_bytesize += log_length;
        self.logs.push(log.into());

        log_length
    }

    #[must_use]
    pub fn last(&self) -> Option<&String> {
        self.logs.last()
    }

    #[must_use]
    pub fn last_message(&self) -> Option<&str> {
        self.logs.last().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_log() {
        let mut bounded_log = LogStream::default();
        let log = b"hello world";
        bounded_log.append(log);
        assert_eq!(Some("hello world"), bounded_log.last_message());
    }

    #[test]
    fn test_display() {
        let mut logs = LogStream::default();
        assert_eq!(String::new(), logs.to_string());

        logs.append(b"hello");
        logs.append(b"world");

        assert_eq!("helloworld", logs.to_string());
    }
}
