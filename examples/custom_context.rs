//! Advanced EasyDRM example with custom per-monitor context
//!
//! Demonstrates:
//! - Creating custom per-monitor context
//! - Initializing resources with GL bindings
//! - Accessing and modifying monitor context
//! - Per-monitor frame counting
//! - Different colors per monitor

use easydrm::{EasyDRM, gl};
use rand::Rng;

/// Custom context that holds per-monitor state
struct MonitorContext {
    frame_count: u32,
    color_offset: f32,
    monitor_name: String,
}

impl MonitorContext {
    /// Initialize context with access to OpenGL bindings
    fn new(gl: &gl::Gles2, _width: usize, _height: usize) -> Self {
        // You could initialize OpenGL resources here if needed
        // For example: VAOs, VBOs, shaders, textures, etc.

        unsafe {
            // Example: query GL version
            let version = std::ffi::CStr::from_ptr(gl.GetString(gl::VERSION) as *const i8);
            println!("Monitor initialized with OpenGL version: {:?}", version);
        }

        let mut rng = rand::rng();

        MonitorContext {
            frame_count: 0,
            color_offset: rng.random_range(0.0..1.0), // Random color offset per monitor
            monitor_name: format!("Monitor-{}", rng.random_range(0..10000)),
        }
    }

    /// Update frame count and calculate color
    fn update(&mut self) -> (f32, f32, f32) {
        self.frame_count += 1;

        // Calculate color based on frame count and offset
        let hue = ((self.frame_count as f32 * 0.01 + self.color_offset) % 1.0).abs();
        hsv_to_rgb(hue, 0.8, 1.0)
    }

    /// Get a status string for this monitor
    fn status(&self) -> String {
        format!("{}: {} frames", self.monitor_name, self.frame_count)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing EasyDRM with custom monitor context...");

    // Initialize EasyDRM with custom context constructor
    // The constructor is called for each monitor with access to its GL bindings
    let mut easydrm = EasyDRM::init(MonitorContext::new)?;

    println!("EasyDRM initialized successfully!");
    println!("Found {} monitor(s)", easydrm.monitor_count());

    if !easydrm.has_monitors() {
        println!("No monitors connected. Waiting for hotplug events...");
    }

    // Print monitor information with their contexts
    for (i, monitor) in easydrm.monitors().enumerate() {
        let mode = monitor.active_mode();
        let ctx = monitor.context();
        println!(
            "Monitor {}: {}x{} @ {}Hz - {}",
            i,
            mode.size().0,
            mode.size().1,
            mode.vrefresh(),
            ctx.monitor_name
        );
    }

    println!("\nStarting render loop. Press Ctrl+C to exit.");
    println!("Each monitor will show a different animated color.\n");

    let mut global_frame_count = 0u64;

    // Main render loop
    loop {
        // Render to each monitor that's ready
        // Use for_each_monitor_mut to get mutable access to contexts
        for monitor in easydrm.monitors_mut() {
            if monitor.can_render() {
                // Make this monitor's OpenGL context current
                if let Ok(_) = monitor.make_current() {
                    // Update context and get color
                    let (r, g, b) = monitor.context_mut().update();

                    // Render with the calculated color
                    let gl = monitor.gl();
                    unsafe {
                        gl.ClearColor(r, g, b, 1.0);
                        gl.Clear(gl::COLOR_BUFFER_BIT);
                    }
                }
            }
        }

        // Swap buffers for all monitors that were drawn to
        easydrm.swap_buffers()?;

        // Poll for events (page flip, hotplug, etc.)
        easydrm.poll_events()?;
        global_frame_count += 1;

        // Print status every 60 frames
        if global_frame_count % 60 == 0 {
            println!("=== Frame {} ===", global_frame_count);
            for (i, monitor) in easydrm.monitors().enumerate() {
                let ctx = monitor.context();
                println!("  Monitor {}: {}", i, ctx.status());
            }
            println!();
        }
    }
}

/// Convert HSV color space to RGB
/// H: 0.0-1.0, S: 0.0-1.0, V: 0.0-1.0
/// Returns (R, G, B) in range 0.0-1.0
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h_prime = h * 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        5 => (c, 0.0, x),
        _ => (c, x, 0.0),
    };

    (r + m, g + m, b + m)
}
