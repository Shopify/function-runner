use colored::Colorize;
use core::fmt;
use std::io;

const MAX_BOUNDED_LOG_BYTESIZE: usize = 1000;

#[derive(Debug)]
pub struct LogStream {
    logs: Vec<String>,
    capacity: usize, // in bytes
    current_bytesize: usize,
}

impl Default for LogStream {
    fn default() -> Self {
        let capacity = MAX_BOUNDED_LOG_BYTESIZE;
        let logs = Vec::new();
        let current_bytesize = 0;
        Self {
            logs,
            capacity,
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
        self.append(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl LogStream {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Append a buffer to the log stream and truncates when hitting the capcity
    /// # Arguments
    /// * `buf` - the buffer to append
    /// # Returns
    /// * `Ok(usize)` - the number of bytes written
    /// * `Err(io::Error)` - if the buffer is empty
    /// # Errors
    /// * `io::Error` - if the buffer is empty
    pub fn append(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.current_bytesize > self.capacity {
            return Ok(buf.len());
        }

        if buf.is_empty() {
            return Ok(0);
        }

        let log = String::from_utf8_lossy(buf);
        let (truncated, log) =
            truncate_to_char_boundary(&log, self.capacity - self.current_bytesize);
        let mut log = log.to_string();
        if truncated {
            log.push_str("...[TRUNCATED]".red().to_string().as_str());
        }

        let size = log.len();

        self.current_bytesize += size;
        self.logs.push(log);

        Ok(buf.len())
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

// truncate `&str` to length at most equal to `max`
// return `true` if it were truncated, and the new str.
fn truncate_to_char_boundary(s: &str, mut max: usize) -> (bool, &str) {
    if max >= s.len() {
        (false, s)
    } else {
        while !s.is_char_boundary(max) {
            max -= 1;
        }
        (true, &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_log() {
        let mut bounded_log = LogStream::with_capacity(15);
        let log = b"hello world";
        bounded_log.append(log).unwrap();
        assert_eq!(Some("hello world"), bounded_log.last_message());
    }

    #[test]
    fn test_bounded_log_when_truncated() {
        let mut bounded_log = LogStream::with_capacity(10);
        let log = b"hello world";
        bounded_log.append(log).unwrap();
        let truncation_message = "...[TRUNCATED]".red().to_string();
        assert_eq!(
            Some(format!("hello worl{}", truncation_message).as_str()),
            bounded_log.last_message()
        );
    }

    #[test]
    fn test_bounded_log_when_truncated_nearest_valid_utf8() {
        let mut bounded_log = LogStream::with_capacity(15);
        bounded_log.append("✌️✌️✌️".as_bytes()).unwrap(); // ✌️ is 6 bytes, ✌ is 3;
        let truncation_message = "...[TRUNCATED]".red().to_string();
        assert_eq!(
            Some(format!("✌\u{fe0f}✌\u{fe0f}✌{}", truncation_message).as_str()),
            bounded_log.last_message()
        );
    }

    #[test]
    fn test_display() {
        let mut logs = LogStream::with_capacity(10);
        assert_eq!(String::new(), logs.to_string());

        logs.append(b"hello").unwrap();
        logs.append(b"world").unwrap();

        assert_eq!("helloworld", logs.to_string());
    }
}
