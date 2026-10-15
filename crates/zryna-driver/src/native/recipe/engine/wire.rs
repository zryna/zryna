use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
use std::os::unix::net::UnixStream;
use std::time::Instant;

use nix::errno::Errno;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::socket::{
    AddressFamily, SockFlag, SockType, UnixAddr, connect, getsockopt, socket, sockopt,
};

use super::types::{Error, remaining};

const SOCKET: &str = "/var/run/docker.sock";

pub(super) struct Wire {
    stream: UnixStream,
    deadline: Instant,
}

pub(super) fn open(deadline: Instant, expected_uid: u32) -> Result<Wire, Error> {
    remaining(deadline)?;
    for parent in ["/", "/var", "/var/run"] {
        let metadata = std::fs::metadata(parent).map_err(|_| Error::Transport)?;
        if !metadata.is_dir()
            || metadata.uid() != 0 && metadata.uid() != expected_uid
            || metadata.mode() & 0o022 != 0
        {
            return Err(Error::Transport);
        }
    }
    let before = std::fs::symlink_metadata(SOCKET).map_err(|_| Error::Transport)?;
    if before.uid() != expected_uid || !before.file_type().is_socket() || before.mode() & 0o002 != 0
    {
        return Err(Error::Transport);
    }
    let fd = socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
        None,
    )
    .map_err(|_| Error::Transport)?;
    let address = UnixAddr::new(SOCKET).map_err(|_| Error::Transport)?;
    match connect(fd.as_raw_fd(), &address) {
        Ok(()) => {}
        Err(Errno::EINPROGRESS) => {
            wait(&fd, PollFlags::POLLOUT, deadline)?;
            if getsockopt(&fd, sockopt::SocketError).map_err(|_| Error::Transport)? != 0 {
                return Err(Error::Transport);
            }
        }
        // AF_UNIX EAGAIN is failed backlog admission, not a pending connection.
        Err(_) => return Err(Error::Transport),
    }
    remaining(deadline)?;
    let after = std::fs::symlink_metadata(SOCKET).map_err(|_| Error::Transport)?;
    let identity = |metadata: &std::fs::Metadata| {
        (
            metadata.dev(),
            metadata.ino(),
            metadata.uid(),
            metadata.gid(),
            metadata.mode(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        )
    };
    if identity(&before) != identity(&after)
        || getsockopt(&fd, sockopt::PeerCredentials).map_err(|_| Error::Transport)?.uid()
            != before.uid()
    {
        return Err(Error::Transport);
    }
    let stream = UnixStream::from(fd);
    if stream.peer_addr().map_err(|_| Error::Transport)?.as_pathname()
        != Some(std::path::Path::new(SOCKET))
    {
        return Err(Error::Transport);
    }
    Ok(Wire { stream, deadline })
}

fn wait(fd: &impl AsFd, flags: PollFlags, deadline: Instant) -> Result<(), Error> {
    loop {
        let left = remaining(deadline)?;
        // Never round a sub-millisecond remainder into a renewed timeout.
        let millis = left.as_millis().min(2_147_483_647);
        let timeout = PollTimeout::try_from(millis).map_err(|_| Error::Transport)?;
        let mut entries = [PollFd::new(fd.as_fd(), flags)];
        let result = poll(&mut entries, timeout);
        remaining(deadline)?;
        match result {
            Ok(0) | Err(Errno::EINTR) => {}
            Ok(_) => {
                let events = entries[0].revents().ok_or(Error::Transport)?;
                if events.contains(PollFlags::POLLNVAL) {
                    return Err(Error::Transport);
                }
                return Ok(());
            }
            Err(_) => return Err(Error::Transport),
        }
    }
}

fn io_failure(error: Error) -> io::Error {
    let kind =
        if error == Error::Deadline { io::ErrorKind::TimedOut } else { io::ErrorKind::Other };
    io::Error::from(kind)
}

impl Read for Wire {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            remaining(self.deadline).map_err(io_failure)?;
            let result = self.stream.read(bytes);
            remaining(self.deadline).map_err(io_failure)?;
            match result {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    wait(&self.stream, PollFlags::POLLIN, self.deadline).map_err(io_failure)?;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                result => return result,
            }
        }
    }
}

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        loop {
            remaining(self.deadline).map_err(io_failure)?;
            let result = self.stream.write(bytes);
            remaining(self.deadline).map_err(io_failure)?;
            match result {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    wait(&self.stream, PollFlags::POLLOUT, self.deadline).map_err(io_failure)?;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                result => return result,
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        remaining(self.deadline).map(|_| ()).map_err(io_failure)
    }
}
