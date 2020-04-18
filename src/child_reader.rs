
use std::io::{self, Read};
use std::process::{Child, ChildStdout, ExitStatus};

/// A wrapper around a +Child+ which forwards +Read+ calls to its stdout, checks
/// for a zero return code on EOF, and reaps the child on +Drop+.
#[derive(Debug)]
pub(crate) struct ChildReader {
    child: Child,
    stdout: Option<ChildStdout>,
}

impl ChildReader {
    pub fn finish(&mut self) -> io::Result<ExitStatus> {
        self.stdout = None;

        self.child.wait()
    }

    fn handle_io<F: FnOnce(&mut ChildStdout) -> io::Result<usize>>(
        &mut self,
        handler: F,
    ) -> io::Result<usize> {
        let res = self
            .stdout
            .as_mut()
            .map(handler)
            .unwrap_or(Err(io::Error::new(io::ErrorKind::BrokenPipe, "Pipe to subprocess closed")));

        if let Ok(0) = res {
            if !self.finish()?.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Subprocess exited with a non-zero return code",
                ))
            }
        }

        res
    }
}

impl From<Child> for ChildReader {
    fn from(mut child: Child) -> Self {
        let stdout = child.stdout.take();

        Self {
            child,
            stdout
        }
    }
}

impl Read for ChildReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.handle_io(|r| r.read(buf))
    }

    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut]) -> io::Result<usize> {
        self.handle_io(|r| r.read_vectored(bufs))
    }
}

impl Drop for ChildReader {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
