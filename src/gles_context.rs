use std::{ffi::CString, num::NonZero, ptr::NonNull};

use drm::control;
use gbm::{AsRaw, BufferObjectFlags, Device as GbmDevice};
use glutin::api::egl;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::ContextAttributesBuilder;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlesContextError {
    #[error("Failed to create EGL display")]
    DisplayCreationFailed,
    #[error("No suitable EGL config found")]
    NoConfigFound,
    #[error("Failed to create GBM surface")]
    GbmSurfaceCreationFailed,
    #[error("Failed to create EGL surface: {0}")]
    EglSurfaceCreationFailed(String),
    #[error("Failed to create EGL context: {0}")]
    EglContextCreationFailed(String),
    #[error("Failed to make context current")]
    MakeCurrentFailed,
}

pub struct GlesContext {
    display: egl::display::Display,
    surface: egl::surface::Surface<WindowSurface>,
    context: egl::context::PossiblyCurrentContext,
    gbm_surface: gbm::Surface<()>,
    gl: crate::gl::Gles2,
}

impl GlesContext {
    /// Creates a new OpenGL ES context for the given monitor mode
    pub fn new(
        gbm_device: &GbmDevice<std::fs::File>,
        mode: &control::Mode,
    ) -> Result<Self, GlesContextError> {
        let (width, height) = mode.size();

        // Create EGL display from GBM device
        let raw_display_handle = RawDisplayHandle::Gbm(GbmDisplayHandle::new(
            NonNull::new(gbm_device.as_raw() as *mut std::ffi::c_void)
                .ok_or(GlesContextError::DisplayCreationFailed)?,
        ));

        let display = unsafe { egl::display::Display::new(raw_display_handle) }
            .map_err(|_| GlesContextError::DisplayCreationFailed)?;

        // Find best EGL config
        let config = find_egl_config(&display)?;

        // Create GBM surface
        let gbm_surface = gbm_device
            .create_surface::<()>(
                width.into(),
                height.into(),
                gbm::Format::Xrgb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .map_err(|_| GlesContextError::GbmSurfaceCreationFailed)?;

        // Create EGL window surface
        let raw_window_handle = RawWindowHandle::Gbm(GbmWindowHandle::new(
            NonNull::new(gbm_surface.as_raw() as *mut std::ffi::c_void)
                .ok_or(GlesContextError::GbmSurfaceCreationFailed)?,
        ));

        let surface = unsafe {
            display
                .create_window_surface(
                    &config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        raw_window_handle,
                        NonZero::new(width as u32)
                            .ok_or(GlesContextError::GbmSurfaceCreationFailed)?,
                        NonZero::new(height as u32)
                            .ok_or(GlesContextError::GbmSurfaceCreationFailed)?,
                    ),
                )
                .map_err(|e| GlesContextError::EglSurfaceCreationFailed(e.to_string()))?
        };

        // Create EGL context
        let context = unsafe {
            display
                .create_context(
                    &config,
                    &ContextAttributesBuilder::new().build(Some(raw_window_handle)),
                )
                .map_err(|e| GlesContextError::EglContextCreationFailed(e.to_string()))?
                .make_current(&surface)
                .map_err(|_| GlesContextError::MakeCurrentFailed)?
        };

        // Load OpenGL function pointers
        let gl = crate::gl::Gles2::load_with(|symbol| {
            let c_symbol = CString::new(symbol).unwrap();
            display.get_proc_address(&c_symbol)
        });

        Ok(GlesContext {
            display,
            surface,
            context,
            gbm_surface,
            gl,
        })
    }

    /// Makes this context current for OpenGL operations
    pub fn make_current(&self) -> Result<(), GlesContextError> {
        self.context
            .make_current(&self.surface)
            .map_err(|_| GlesContextError::MakeCurrentFailed)?;
        Ok(())
    }

    /// Swaps buffers and returns the new buffer object for presentation
    pub fn swap_buffers(&mut self) -> Result<gbm::BufferObject<()>, GlesContextError> {
        // Swap EGL buffers
        self.surface
            .swap_buffers(&self.context)
            .map_err(|_| GlesContextError::MakeCurrentFailed)?;

        // Lock front buffer from GBM surface
        let bo = unsafe { self.gbm_surface.lock_front_buffer() }
            .map_err(|_| GlesContextError::GbmSurfaceCreationFailed)?;

        Ok(bo)
    }

    /// Gets a function pointer for loading OpenGL functions
    pub fn get_proc_address(&self, symbol: &str) -> *const std::ffi::c_void {
        let c_symbol = CString::new(symbol).unwrap();
        self.display.get_proc_address(&c_symbol)
    }

    /// Gets a reference to the OpenGL ES bindings
    pub fn gl(&self) -> &crate::gl::Gles2 {
        &self.gl
    }

    /// Gets a reference to the EGL display for advanced operations (like fences)
    pub(crate) fn display(&self) -> &egl::display::Display {
        &self.display
    }
}

// GlesContext uses RAII - all fields are automatically dropped

/// Finds the best EGL config with the highest number of samples
fn find_egl_config(
    display: &egl::display::Display,
) -> Result<egl::config::Config, GlesContextError> {
    unsafe { display.find_configs(ConfigTemplateBuilder::new().build()) }
        .map_err(|_| GlesContextError::NoConfigFound)?
        .reduce(|config, acc| {
            if config.num_samples() > acc.num_samples() {
                config
            } else {
                acc
            }
        })
        .ok_or(GlesContextError::NoConfigFound)
}
