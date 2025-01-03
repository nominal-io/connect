use bevy::prelude::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAX_FLIGHT_STREAM_POINTS: usize = 10_000;
pub const MAX_CHANNEL_STREAM_POINTS: usize = 100;

#[derive(Clone)]
pub enum StreamPoint {
    Plot2D([f64; 2]),
    FlightData([f64; 6]),
}

impl StreamPoint {
    // Get 2D coordinates (for basic plotting)
    pub fn as_plot2d(&self) -> Option<[f64; 2]> {
        match self {
            StreamPoint::Plot2D(coords) => Some(*coords),
            StreamPoint::FlightData([lat, lon, ..]) => Some([*lat, *lon]),
        }
    }

    // Get all flight data
    pub fn as_flight_data(&self) -> Option<[f64; 6]> {
        match self {
            StreamPoint::FlightData(data) => Some(*data),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessStatus {
    Running,
    Failed(Option<i32>), // Exit code
    Finished,            // Finished execution itself
    Stopped,             // Stopped by this process
}

#[derive(Resource)]
pub struct StreamManager {
    pub streams: Arc<Mutex<HashMap<String, Vec<StreamPoint>>>>,
    running: Arc<Mutex<bool>>,
    receiver: Receiver<StreamData>,
    _sender: Sender<StreamData>,
    streaming_processes: Arc<Mutex<Vec<Child>>>,
    pub process_statuses: Arc<Mutex<Vec<ProcessStatus>>>,
    pub debug: bool,
}

#[derive(Clone, Deserialize, Debug)]
pub struct StreamData {
    pub stream_id: String,
    pub timestamp: f64,
    #[serde(default)]
    pub value: f64, // For 2D line plots
    #[serde(default)]
    pub rel_lat: f64, // For 3D position
    #[serde(default)]
    pub rel_lon: f64, // For 3D position
    #[serde(default)]
    pub altitude: f64, // Aircraft altitude
    #[serde(default)]
    pub pitch: f64, // Aircraft pitch angle
    #[serde(default)]
    pub roll: f64, // Aircraft roll angle
    #[serde(default)]
    pub yaw: f64, // Aircraft yaw angle
}

impl StreamManager {
    pub fn new(debug: bool) -> Self {
        let (sender, receiver) = bounded(MAX_FLIGHT_STREAM_POINTS);
        let running = Arc::new(Mutex::new(false));
        let running_clone = Arc::clone(&running);
        let debug = debug;
        let sender_clone = sender.clone();

        // Spawn ZMQ listener thread
        thread::spawn(move || {
            debug!("Starting ZMQ listener thread");
            let context = zmq::Context::new();
            let subscriber = match context.socket(zmq::PULL) {
                Ok(s) => {
                    debug!("Successfully created ZMQ PULL socket");
                    s
                }
                Err(e) => {
                    debug!("Failed to create ZMQ socket: {:?}", e);
                    return;
                }
            };

            debug!("Setting ZMQ socket options...");

            // Add a small receive timeout to help with debugging
            if let Err(e) = subscriber.set_rcvtimeo(100) {
                debug!("Failed to set receive timeout: {:?}", e);
            }

            debug!("Connecting to tcp://localhost:5555");

            if let Err(e) = subscriber.connect("tcp://localhost:5555") {
                debug!("Failed to connect: {:?}", e);
                debug!("Is the Python script running and binding to port 5555?");
                return;
            } else {
                debug!("Successfully connected to tcp://localhost:5555");
            }

            debug!("ZMQ socket setup complete, entering main loop");

            loop {
                let is_running = running_clone
                    .lock()
                    .map(|guard| *guard)
                    .unwrap_or_else(|e| {
                        debug!("Failed to lock running state: {:?}", e);
                        false
                    });

                if !is_running {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                debug!("Attempting to receive ZMQ message...");
                match subscriber.recv_string(zmq::DONTWAIT) {
                    Ok(Ok(msg)) => {
                        debug!("ZMQ received raw message: {}", msg);
                        debug!("Message length: {} bytes", msg.len());
                        match serde_json::from_str::<StreamData>(&msg) {
                            Ok(data) => {
                                debug!("Successfully parsed message: {:?}", data);
                                if sender_clone.send(data).is_err() {
                                    debug!("Failed to send data through channel");
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse message: {:?}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        debug!("Invalid UTF8 in message: {:?}", e);
                    }
                    Err(e) => {
                        if e != zmq::Error::EAGAIN {
                            debug!("ZMQ receive error: {:?}", e);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
            running,
            receiver,
            _sender: sender,
            streaming_processes: Arc::new(Mutex::new(Vec::new())),
            process_statuses: Arc::new(Mutex::new(Vec::new())),
            debug,
        }
    }

    pub fn start_streaming(&mut self) {
        // Kill any existing streaming processes
        if let Ok(mut processes) = self.streaming_processes.lock() {
            for process in processes.iter_mut() {
                let _ = process.kill();
            }
            processes.clear();
        }

        // Clear existing streams
        if let Ok(mut streams) = self.streams.lock() {
            streams.clear();
        }

        debug!("Setting running to true");
        if let Ok(mut running) = self.running.lock() {
            *running = true;
            info!(
                "Running state set to: {} (address: {:p})",
                *running, &self.running
            );
        } else {
            warn!("Failed to set running state");
        }
    }

    pub fn stop_streaming(&mut self) {
        debug!("Setting running to false");
        if let Ok(mut running) = self.running.lock() {
            *running = false;
            info!(
                "Running state set to: {} (address: {:p})",
                *running, &self.running
            );
        }

        // Kill all streaming processes and mark them as stopped
        if let Ok(mut processes) = self.streaming_processes.lock() {
            for process in processes.iter_mut() {
                let _ = process.kill();
            }
            processes.clear();

            // Mark all processes as stopped
            if let Ok(mut statuses) = self.process_statuses.lock() {
                for status in statuses.iter_mut() {
                    *status = ProcessStatus::Stopped;
                }
            }
        }

        // Clear the streams data
        if let Ok(mut streams) = self.streams.lock() {
            streams.clear();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().map(|guard| *guard).unwrap_or(false)
    }

    pub fn add_streaming_process(&mut self, mut child: Child) {
        // Redirect stdout to capture Python script output
        if let Some(stdout) = child.stdout.take() {
            let stdout_reader = BufReader::new(stdout);
            thread::spawn(move || {
                for line in stdout_reader.lines() {
                    if let Ok(line) = line {
                        info!("Python output: {}", line);
                    }
                }
            });
        }

        // Store the process and its initial status
        if let Ok(mut processes) = self.streaming_processes.lock() {
            processes.push(child);
            if let Ok(mut statuses) = self.process_statuses.lock() {
                statuses.push(ProcessStatus::Stopped);
            }
        }
    }
}

pub fn update_streams(stream_manager: ResMut<StreamManager>) {
    if !*stream_manager.running.lock().unwrap() {
        return;
    }

    debug!("Checking for new stream data...");

    while let Ok(data) = stream_manager.receiver.try_recv() {
        debug!("Received data for stream: {}", data.stream_id);

        if let Ok(mut streams) = stream_manager.streams.lock() {
            match data.stream_id.as_str() {
                "single_scalar_channel" => {
                    let points = streams.entry(data.stream_id).or_default();
                    points.push(StreamPoint::Plot2D([data.timestamp, data.value]));
                    if points.len() > MAX_CHANNEL_STREAM_POINTS {
                        points.remove(0);
                    }
                }
                "flight_position" => {
                    let points = streams.entry(data.stream_id).or_default();
                    points.push(StreamPoint::FlightData([
                        data.rel_lat,
                        data.rel_lon,
                        data.altitude,
                        data.pitch,
                        data.roll,
                        data.yaw,
                    ]));
                    if points.len() > MAX_FLIGHT_STREAM_POINTS {
                        points.remove(0);
                    }
                    if stream_manager.debug
                        && (data.altitude == 0.0
                            || data.pitch == 0.0
                            || data.roll == 0.0
                            || data.yaw == 0.0)
                    {
                        debug!(
                            "Warning: Some flight data fields were missing and defaulted to 0.0"
                        );
                    }
                }
                _ => {
                    debug!("Unknown stream_id: {}", data.stream_id);
                }
            }
        }
    }
}

pub fn check_process_status(stream_manager: ResMut<StreamManager>) {
    if let (Ok(mut processes), Ok(mut statuses)) = (
        stream_manager.streaming_processes.lock(),
        stream_manager.process_statuses.lock(),
    ) {
        for (i, process) in processes.iter_mut().enumerate() {
            if i >= statuses.len() {
                continue;
            }

            match process.try_wait() {
                Ok(Some(status)) => {
                    // Process has finished
                    if status.success() {
                        statuses[i] = ProcessStatus::Finished;
                    } else {
                        statuses[i] = ProcessStatus::Failed(status.code());
                    }
                }
                Ok(None) => {
                    // Process is still running
                    statuses[i] = ProcessStatus::Running;
                }
                Err(e) => {
                    error!("Error checking process status: {:?}", e);
                    // Error checking process status, assume the script isn't running
                    statuses[i] = ProcessStatus::Failed(None);
                }
            }
        }
    }
}
