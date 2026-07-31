// Standalone RPLIDAR A2 smoke test.
// Goal: prove the hardware + driver work BEFORE any Mech integration.
// Your LiDAR is on /dev/ttyUSB1 (ttyUSB0 is the Yahboom robot).

use rplidar_drv::RplidarDevice;
use std::time::Duration;

fn main() {
    let port = std::env::args().nth(1).unwrap_or_else(|| "/dev/ttyUSB1".to_string());
    println!("Opening {port} at 115200 baud (A2M8)...");

    // A2M8 uses 115200. (A3 / S-series use 256000 — not your unit.)
    let serial = serialport::new(&port, 115200)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("failed to open serial port (check port + dialout group)");

    let mut lidar = RplidarDevice::with_stream(serial);

    match lidar.get_device_info() {
        Ok(info) => println!(
            "RPLIDAR model={} firmware={}.{} hardware={}",
            info.model,
            info.firmware_version >> 8,
            info.firmware_version & 0xff,
            info.hardware_version
        ),
        Err(e) => {
            eprintln!("get_device_info failed: {e:?}");
            eprintln!("Is this the LiDAR port? Try /dev/ttyUSB0. Is the motor spinning?");
            std::process::exit(1);
        }
    }

    lidar.start_motor().ok();               // some adapters auto-start; harmless if so
    lidar.start_scan().expect("start_scan failed");
    println!("Scanning. Showing 40 points (angle / distance / quality):\n");

    let mut shown = 0;
    while shown < 40 {
        match lidar.grab_scan_point() {
            Ok(p) => {
                // NOTE: distance units vary by crate version.
                // rplidar_drv 0.6 returns METERS from p.distance().
                // If your numbers look ~1000x off, drop or add the *1000.
                let dist_mm = p.distance() * 1000.0;
                let angle_deg = p.angle() * 180.0 / std::f32::consts::PI;
                println!("angle={angle_deg:7.2}°   dist={dist_mm:8.1} mm   quality={}", p.quality);
                shown += 1;
            }
            Err(e) => {
                eprintln!("grab_scan_point error: {e:?}");
                break;
            }
        }
    }

    lidar.stop().ok();
    lidar.stop_motor().ok();
    println!("\nSmoke test done. If you saw sane distances, hardware is GOOD.");
}
