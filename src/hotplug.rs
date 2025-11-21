use std::os::fd::{AsRawFd, OwnedFd};

use nix::errno::Errno;
use nix::sys::socket::*;

pub(crate) struct UEventSocket {
    pub(crate) fd: OwnedFd,
}

impl UEventSocket {
    pub(crate) fn open() -> nix::Result<Self> {
        // criar socket netlink para uevents do kernel
        let fd = socket(
            AddressFamily::Netlink,
            SockType::Datagram,
            SockFlag::empty(),
            Some(SockProtocol::NetlinkKObjectUEvent),
        )?;

        // subscrever o grupo 1 (kernel uevents)
        let addr = NetlinkAddr::new(0, 1);
        bind(fd.as_raw_fd(), &addr)?;
        Ok(Self { fd })
    }
    pub(crate) fn drain_hotplug_events(&self) -> nix::Result<bool> {
        let mut buf = [0u8; 4096];
        let mut found = false;

        loop {
            match recv(self.fd.as_raw_fd(), &mut buf, MsgFlags::MSG_DONTWAIT) {
                Ok(len) if len > 0 => {
                    let msg = std::str::from_utf8(&buf[..len]).unwrap_or("");
                    let is_drm_hotplug = msg.contains("SUBSYSTEM=drm")
                        && msg.contains("HOTPLUG=1")
                        && (msg.contains("DEVTYPE=connector") || msg.contains("DEVNAME=card"));

                    if is_drm_hotplug {
                        found = true;
                    }
                }
                Ok(_) => continue,
                Err(Errno::EAGAIN) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(found)
    }
}
