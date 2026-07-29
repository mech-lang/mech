//! Native (hardware) LiDAR host: owns a Slamtec RPLIDAR over serial, reads
//! scans on a background thread, reduces each sweep to a `LidarSnapshot`, and
//! publishes it as a runtime host-input packet.
//!
//! The threading / lifecycle logic (attach, start, stop, restart-after-error,
//! back-pressure) mirrors the `time` host's `NativeTimeInputDriver`. The only
//! LiDAR-specific part is `read_snapshot`, which drives the RPLIDAR and reduces
//! a rotation of points to a few scalars.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use mech_core::MResult;
use mech_runtime::{
    materialize_host_manifest, ConfigValue, HostManifestConfig, RuntimeHostFactory,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInstallation, RuntimeIngress,
};

use rplidar_drv::RplidarDevice;
use serialport::SerialPort;

use crate::{
    lidar_error, lidar_host_manifest, lidar_settings_from_config, lidar_source_matches,
    new_shared_snapshot, LidarHostSettings, LidarResourceProvider, LidarSnapshot,
    SharedLidarSnapshot,
};

/// Concrete device type: an RPLIDAR speaking over a boxed serial port.
type LidarDev = RplidarDevice<Box<dyn SerialPort>>;
/// The device is shared between the driver thread and shutdown code.
type SharedDevice = Arc<Mutex<LidarDev>>;

/// Open the serial port, construct the RPLIDAR device, and start scanning.
/// Called once at instantiate time, when the port is known from settings.
fn open_device(settings: &LidarHostSettings) -> MResult<SharedDevice> {
    let serial = serialport::new(&settings.port, settings.baud)
        .timeout(Duration::from_millis(2000))
        .open()
        .map_err(|e| lidar_error("LidarOpen", format!("cannot open {}: {e}", settings.port)))?;

    let mut dev = RplidarDevice::with_stream(serial);
    // Best-effort motor start (some USB adapters auto-drive the motor).
    let _ = dev.start_motor();
    dev.start_scan()
        .map_err(|e| lidar_error("LidarStartScan", format!("start_scan failed: {e:?}")))?;
    Ok(Arc::new(Mutex::new(dev)))
}

/// Read one reduced snapshot: grab `points_per_scan` points (~one rotation for
/// the A2) and reduce them to nearest / front / count. This is the only
/// hardware-specific function in the driver.
fn read_snapshot(dev: &SharedDevice, points: usize, scan_id: u64) -> MResult<LidarSnapshot> {
    let mut guard = dev
        .lock()
        .map_err(|_| lidar_error("LidarRead", "lidar device lock is poisoned"))?;

    let mut nearest = f64::INFINITY;
    let mut nearest_angle = 0.0_f64;
    let mut front = f64::INFINITY;
    let mut valid = 0.0_f64;

    for _ in 0..points {
        match guard.grab_scan_point() {
            Ok(p) => {
                if p.quality == 0 {
                    continue;
                }
                // rplidar_drv 0.6 returns METERS; convert to mm.
                // If your distances look 1000x off, adjust this one line.
                let dist_mm = (p.distance() * 1000.0) as f64;
                let angle_deg = (p.angle() * 180.0 / std::f32::consts::PI) as f64;
                if dist_mm <= 0.0 {
                    continue;
                }
                valid += 1.0;
                if dist_mm < nearest {
                    nearest = dist_mm;
                    nearest_angle = angle_deg;
                }
                // "front" = points within +/- 10 degrees of straight ahead (0/360).
                let ahead = angle_deg <= 10.0 || angle_deg >= 350.0;
                if ahead && dist_mm < front {
                    front = dist_mm;
                }
            }
            Err(_) => {
                // Transient read error / timeout: stop this batch, publish what
                // we have. The next interval tries again.
                break;
            }
        }
    }

    Ok(LidarSnapshot {
        nearest_mm: if nearest.is_finite() { nearest } else { 0.0 },
        nearest_angle,
        front_mm: if front.is_finite() { front } else { 0.0 },
        count: valid,
        scan_id: scan_id as f64,
    })
}

struct WorkerLiveReset(Arc<AtomicBool>);
impl Drop for WorkerLiveReset {
    fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); }
}

pub struct NativeLidarInputDriver {
    instance: String,
    dev: SharedDevice,
    snapshot: SharedLidarSnapshot,
    ingress: Arc<Mutex<Option<RuntimeIngress>>>,
    live: Arc<AtomicBool>,
    interval: Duration,
    points_per_scan: usize,
    scan_counter: Arc<AtomicU64>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
    stop_sender: Arc<Mutex<Option<Sender<()>>>>,
}

impl std::fmt::Debug for NativeLidarInputDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeLidarInputDriver")
            .field("instance", &self.instance)
            .field("live", &self.is_live())
            .field("interval", &self.interval)
            .field("points_per_scan", &self.points_per_scan)
            .finish_non_exhaustive()
    }
}

impl NativeLidarInputDriver {
    pub fn new(
        instance: impl Into<String>,
        dev: SharedDevice,
        snapshot: SharedLidarSnapshot,
        interval: Duration,
        points_per_scan: usize,
    ) -> Self {
        Self {
            instance: instance.into(),
            dev,
            snapshot,
            ingress: Arc::new(Mutex::new(None)),
            live: Arc::new(AtomicBool::new(false)),
            interval,
            points_per_scan,
            scan_counter: Arc::new(AtomicU64::new(0)),
            worker: Arc::new(Mutex::new(None)),
            stop_sender: Arc::new(Mutex::new(None)),
        }
    }

    fn prepare_worker_start(&mut self) -> MResult<bool> {
        let mut stop_guard = self
            .stop_sender
            .lock()
            .map_err(|_| lidar_error("LidarDriverStart", "lidar stop-signal lock is poisoned"))?;
        let mut worker_guard = self
            .worker
            .lock()
            .map_err(|_| lidar_error("LidarDriverStart", "lidar worker lock is poisoned"))?;

        if self.live.load(Ordering::SeqCst)
            && worker_guard.as_ref().is_some_and(|h| !h.is_finished())
        {
            return Ok(true);
        }

        let stop_sender = stop_guard.take();
        let worker = worker_guard.take();
        drop(worker_guard);
        drop(stop_guard);

        self.live.store(false, Ordering::SeqCst);
        if let Some(sender) = stop_sender {
            let _ = sender.send(());
        }
        if let Some(handle) = worker {
            handle
                .join()
                .map_err(|_| lidar_error("LidarDriverStart", "lidar worker panicked before restart"))?;
        }
        Ok(false)
    }
}

impl RuntimeHostInputDriver for NativeLidarInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        lidar_source_matches(&self.instance, source)
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        if self.is_live() {
            return Err(lidar_error("LidarDriverAttach", "cannot attach lidar driver while live"));
        }
        let mut guard = self
            .ingress
            .lock()
            .map_err(|_| lidar_error("LidarDriverAttach", "lidar ingress lock is poisoned"))?;
        if guard.is_some() {
            return Err(lidar_error("LidarDriverAttach", "lidar driver is already attached"));
        }
        *guard = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        if self.prepare_worker_start()? {
            return Ok(());
        }
        let ingress = self
            .ingress
            .lock()
            .map_err(|_| lidar_error("LidarDriverStart", "lidar ingress lock is poisoned"))?
            .clone()
            .ok_or_else(|| lidar_error("LidarDriverStart", "lidar driver must be attached before start"))?;

        let (stop_sender, stop_receiver) = mpsc::channel();
        *self
            .stop_sender
            .lock()
            .map_err(|_| lidar_error("LidarDriverStart", "lidar stop-signal lock is poisoned"))? =
            Some(stop_sender);
        self.live.store(true, Ordering::SeqCst);

        let live = self.live.clone();
        let dev = self.dev.clone();
        let snapshot = self.snapshot.clone();
        let interval = self.interval;
        let points = self.points_per_scan;
        let instance = self.instance.clone();
        let counter = self.scan_counter.clone();

        let worker = thread::spawn(move || {
            let _live_reset = WorkerLiveReset(live.clone());
            while live.load(Ordering::SeqCst) {
                let scan_id = counter.fetch_add(1, Ordering::SeqCst);
                match read_snapshot(&dev, points, scan_id) {
                    Ok(next) => {
                        if let Ok(mut g) = snapshot.lock() {
                            *g = next;
                        }
                        match next.into_host_input(&instance).and_then(|pkt| ingress.submit(pkt)) {
                            Ok(()) => {}
                            Err(err) => match err.kind_name().as_str() {
                                "RuntimeIngressFull" => { /* skip; try next interval */ }
                                "RuntimeIngressClosed" => {
                                    live.store(false, Ordering::SeqCst);
                                    break;
                                }
                                _ => {
                                    live.store(false, Ordering::SeqCst);
                                    break;
                                }
                            },
                        }
                    }
                    Err(_) => {
                        // hardware read failed hard: stop cleanly
                        live.store(false, Ordering::SeqCst);
                        break;
                    }
                }
                match stop_receiver.recv_timeout(interval) {
                    Ok(()) => break,
                    Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        });

        *self
            .worker
            .lock()
            .map_err(|_| lidar_error("LidarDriverStart", "lidar worker lock is poisoned"))? =
            Some(worker);
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        let stop_sender = self
            .stop_sender
            .lock()
            .map_err(|_| lidar_error("LidarDriverStop", "lidar stop-signal lock is poisoned"))?
            .take();
        if let Some(sender) = stop_sender {
            let _ = sender.send(());
        }
        let handle = self
            .worker
            .lock()
            .map_err(|_| lidar_error("LidarDriverStop", "lidar worker lock is poisoned"))?
            .take();
        if let Some(handle) = handle {
            handle
                .join()
                .map_err(|_| lidar_error("LidarDriverStop", "lidar worker panicked during shutdown"))?;
        }
        Ok(())
    }

    fn is_live(&self) -> bool { self.live.load(Ordering::SeqCst) }
}

impl Drop for NativeLidarInputDriver {
    fn drop(&mut self) {
        let _ = self.stop();
        if let Ok(mut guard) = self.dev.lock() {
            let _ = guard.stop();
            let _ = guard.stop_motor();
        }
    }
}

#[derive(Debug)]
pub struct NativeLidarHostFactory {
    manifest: HostManifestConfig,
}

impl NativeLidarHostFactory {
    pub fn new() -> MResult<Self> {
        Ok(Self { manifest: lidar_host_manifest()? })
    }
}

impl RuntimeHostFactory for NativeLidarHostFactory {
    fn provider_name(&self) -> &str { "lidar" }

    fn manifest(&self) -> &HostManifestConfig { &self.manifest }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        lidar_settings_from_config(settings).map(|_| ())
    }

    fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
        let settings = lidar_settings_from_config(settings)?;
        let dev = open_device(&settings)?;
        let snapshot = new_shared_snapshot(LidarSnapshot::default());
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(LidarResourceProvider::new(
                instance_name,
                snapshot.clone(),
            ))],
            input_drivers: vec![Box::new(NativeLidarInputDriver::new(
                instance_name,
                dev,
                snapshot,
                Duration::from_millis(settings.interval_ms),
                settings.points_per_scan,
            ))],
        })
    }
}
