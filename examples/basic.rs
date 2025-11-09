//! Basic EasyDRM example
//!
//! Demonstrates:
//! - Initializing EasyDRM without custom context
//! - Rendering loop with event polling
//! - Basic OpenGL rendering (clearing screen)
//! - Multi-monitor support
//! - Graceful handling of monitor hotplug

use easydrm::{EasyDRM, gl};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing EasyDRM...");

    // Initialize EasyDRM without custom per-monitor context
    let mut easydrm = EasyDRM::init_empty()?;

    println!("EasyDRM initialized successfully!");
    println!("Found {} monitor(s)", easydrm.monitor_count());

    if !easydrm.has_monitors() {
        println!("No monitors connected. Waiting for hotplug events...");
    }

    // Print monitor information
    for (i, monitor) in easydrm.monitors().enumerate() {
        let mode = monitor.active_mode();
        println!(
            "Monitor {}: {}x{} @ {}Hz",
            i,
            mode.size().0,
            mode.size().1,
            mode.vrefresh()
        );
    }

    println!("\nStarting render loop. Press Ctrl+C to exit.");

    let mut frame_count = 0u64;

    // Main render loop
    loop {
        // Render to each monitor that's ready
        for monitor in easydrm.monitors_mut() {
            if monitor.can_render() {
                // Make this monitor's OpenGL context current
                if let Ok(_) = monitor.make_current() {
                    // Get OpenGL bindings
                    let gl = monitor.gl();

                    // Simple animation: cycle through colors based on frame count
                    let hue = (frame_count % 360) as f32 / 360.0;
                    let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);

                    unsafe {
                        // Clear the screen with animated color
                        gl.ClearColor(r, g, b, 1.0);
                        gl.Clear(gl::COLOR_BUFFER_BIT);
                    }
                }
            }
        }

        // Swap buffers for all monitors that were drawn to
        easydrm.swap_buffers()?;

        // Poll for events (page flip, hotplug, etc.)
        // This blocks until an event is received
        easydrm.poll_events()?;
        frame_count += 1;

        // Print status every 60 frames
        if frame_count % 60 == 0 {
            println!(
                "Frame {}: {} monitor(s) active",
                frame_count,
                easydrm.monitor_count()
            );
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
