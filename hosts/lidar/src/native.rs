use std::io::Write;
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

//type LidarDev = RplidarDevice<Box<dyn SerialPort>>;
type LidarDev = RplidarDevice<dyn SerialPort>;
type SharedDevice = Arc<Mutex<LidarDev>>;

fn open_device(settings: &LidarHostSettings) -> MResult<SharedDevice> {
    let mut serial = serialport::new(&settings.port, settings.baud)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| lidar_error("LidarOpen", format!("cannot open {}: {e}", settings.port)))?;

    // Send raw STOP to clear any previous scan state
    serial.write_all(&[0xA5, 0x25]).ok();
    serial.flush().ok();
    std::thread::sleep(Duration::from_millis(500));

    // Set working timeout
    serial.set_timeout(Duration::from_millis(500)).ok();

    // Start motor via DTR
    serial.write_data_terminal_ready(true).ok();

    let mut dev = RplidarDevice::with_stream(serial);
    dev.set_motor_pwm(660).ok();
    std::thread::sleep(Duration::from_secs(2));

    dev.start_scan()
        .map_err(|e| lidar_error("LidarStartScan", format!("start_scan failed: {e:?}")))?;
    std::thread::sleep(Duration::from_millis(500));

    // Skip first batch of potentially stale points
    for _ in 0..20 {
        dev.grab_scan_point().ok();
    }

    Ok(Arc::new(Mutex::new(dev)))
}

fn read_snapshot(dev: &SharedDevice, points: usize, scan_id: u64) -> MResult<LidarSnapshot> {
    let mut guard = dev.lock()
        .map_err(|_| lidar_error("LidarRead", "device lock poisoned"))?;

    let mut nearest = f64::INFINITY;
    let mut nearest_angle = 0.0_f64;
    let mut front = f64::INFINITY;
    let mut valid = 0.0_f64;
    let mut timeouts = 0;

    for _ in 0..points {
        match guard.grab_scan_point() {
            Ok(p) => {
                if p.quality == 0 { continue; }
                let dist_mm = (p.distance() as f64) * 1000.0;
                let angle_deg = (p.angle() as f64) * 180.0 / std::f64::consts::PI;
                if dist_mm <= 0.0 { continue; }
                valid += 1.0;
                timeouts = 0;
                if dist_mm < nearest {
                    nearest = dist_mm;
                    nearest_angle = angle_deg;
                }
                let ahead = angle_deg <= 10.0 || angle_deg >= 350.0;
                if ahead && dist_mm < front {
                    front = dist_mm;
                }
            }
            Err(_) => {
                timeouts += 1;
                if timeouts >= 3 {
                    // Scan stream died — restart it
                    guard.stop().ok();
                    std::thread::sleep(Duration::from_millis(100));
                    guard.start_scan().ok();
                    std::thread::sleep(Duration::from_millis(300));
                    timeouts = 0;
                }
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
}

impl RuntimeHostInputDriver for NativeLidarInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        lidar_source_matches(&self.instance, source)
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut guard = self.ingress.lock()
            .map_err(|_| lidar_error("LidarAttach", "ingress lock poisoned"))?;
        *guard = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        if self.live.load(Ordering::SeqCst) { return Ok(()); }

        let ingress = self.ingress.lock()
            .map_err(|_| lidar_error("LidarStart", "ingress lock poisoned"))?
            .clone()
            .ok_or_else(|| lidar_error("LidarStart", "must attach before start"))?;

        let (stop_tx, stop_rx) = mpsc::channel();
        *self.stop_sender.lock().unwrap() = Some(stop_tx);
        self.live.store(true, Ordering::SeqCst);

        let live = self.live.clone();
        let dev = self.dev.clone();
        let snapshot = self.snapshot.clone();
        let interval = self.interval;
        let points = self.points_per_scan;
        let instance = self.instance.clone();
        let counter = self.scan_counter.clone();

        let worker = thread::spawn(move || {
            let _reset = WorkerLiveReset(live.clone());
            while live.load(Ordering::SeqCst) {
                let id = counter.fetch_add(1, Ordering::SeqCst);
                match read_snapshot(&dev, points, id) {
                    Ok(next) => {
                        if let Ok(mut g) = snapshot.lock() { *g = next; }
                        match next.into_host_input(&instance).and_then(|pkt| ingress.submit(pkt)) {
                            Ok(()) => {}
                            Err(err) => {
                                if err.kind_name() == "RuntimeIngressClosed" {
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
                match stop_rx.recv_timeout(interval) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        });

        *self.worker.lock().unwrap() = Some(worker);
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        if let Some(tx) = self.stop_sender.lock().unwrap().take() { let _ = tx.send(()); }
        if let Some(h) = self.worker.lock().unwrap().take() { let _ = h.join(); }
        Ok(())
    }

    fn is_live(&self) -> bool { self.live.load(Ordering::SeqCst) }
}

impl Drop for NativeLidarInputDriver {
    fn drop(&mut self) {
        let _ = self.stop();
        if let Ok(mut g) = self.dev.lock() {
            g.stop().ok();
            g.set_motor_pwm(0).ok();
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
    fn validate_settings(&self, _name: &str, settings: &ConfigValue) -> MResult<()> {
        lidar_settings_from_config(settings).map(|_| ())
    }
    fn instantiate(&self, name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
        let settings = lidar_settings_from_config(settings)?;
        let dev = open_device(&settings)?;
        let snapshot = new_shared_snapshot(LidarSnapshot::default());
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(name, &self.manifest)?,
            resource_providers: vec![Box::new(LidarResourceProvider::new(name, snapshot.clone()))],
            input_drivers: vec![Box::new(NativeLidarInputDriver::new(
                name, dev, snapshot,
                Duration::from_millis(settings.interval_ms),
                settings.points_per_scan,
            ))],
        })
    }
}
