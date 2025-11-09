use drm::Device;
use drm::control::Device as ControlDevice;
use glutin::api::egl;
use std::os::unix::io::AsRawFd;

#[derive(Debug)]
/// A simple wrapper for a device node.
pub struct Card(std::fs::File);

/// Implementing `AsFd` is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling `as_fd()` on the inner File.
impl std::os::unix::io::AsFd for Card {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for Card {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.0.as_raw_fd()
    }
}

/// With `AsFd` implemented, we can now implement `drm::Device`.
impl Device for Card {}
impl ControlDevice for Card {}

impl Card {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);
        Ok(Self(options.open(path)?))
    }
    pub fn open_default_card() -> Self {
        let gpus = egl::device::Device::query_devices()
            .expect("Failed to query devices")
            .filter_map(|egl_device| {
                egl_device
                    .drm_device_node_path()
                    .and_then(|p| p.as_os_str().to_str())
            });
        for gpu_file_path in gpus {
            match Self::open(gpu_file_path) {
                Ok(card) => return card,
                Err(err) => eprintln!("Error while opening card {gpu_file_path}: {err}"),
            }
        }
        panic!();
    }
}
