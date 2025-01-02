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

#[derive(Resource)]
pub struct StreamManager {
    pub streams: Arc<Mutex<HashMap<String, Vec<StreamPoint>>>>,
    running: Arc<Mutex<bool>>,
    receiver: Receiver<StreamData>,
    _sender: Sender<StreamData>,
    streaming_processes: Arc<Mutex<Vec<Child>>>,
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
            if debug {
                println!("Starting ZMQ listener thread");
            }
            let context = zmq::Context::new();
            let subscriber = match context.socket(zmq::PULL) {
                Ok(s) => {
                    if debug {
                        println!("Successfully created ZMQ PULL socket");
                    }
                    s
                }
                Err(e) => {
                    if debug {
                        println!("Failed to create ZMQ socket: {:?}", e);
                    }
                    return;
                }
            };

            if debug {
                println!("Setting ZMQ socket options...");
            }

            // Add a small receive timeout to help with debugging
            if let Err(e) = subscriber.set_rcvtimeo(100) {
                if debug {
                    println!("Failed to set receive timeout: {:?}", e);
                }
            }

            if debug {
                println!("Connecting to tcp://localhost:5555");
            }

            if let Err(e) = subscriber.connect("tcp://localhost:5555") {
                if debug {
                    println!("Failed to connect: {:?}", e);
                    println!("Is the Python script running and binding to port 5555?");
                }
                return;
            } else {
                if debug {
                    println!("Successfully connected to tcp://localhost:5555");
                }
            }

            if debug {
                println!("ZMQ socket setup complete, entering main loop");
            }

            loop {
                let is_running = running_clone
                    .lock()
                    .map(|guard| *guard)
                    .unwrap_or_else(|e| {
                        if debug {
                            println!("Failed to lock running state: {:?}", e);
                        }
                        false
                    });

                if !is_running {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                if debug {
                    println!("Attempting to receive ZMQ message...");
                }
                match subscriber.recv_string(zmq::DONTWAIT) {
                    Ok(Ok(msg)) => {
                        if debug {
                            println!("ZMQ received raw message: {}", msg);
                            println!("Message length: {} bytes", msg.len());
                        }
                        match serde_json::from_str::<StreamData>(&msg) {
                            Ok(data) => {
                                if debug {
                                    println!("Successfully parsed message: {:?}", data);
                                }
                                if sender_clone.send(data).is_err() {
                                    if debug {
                                        println!("Failed to send data through channel");
                                    }
                                    break;
                                }
                            }
                            Err(e) => {
                                if debug {
                                    println!("Failed to parse message: {:?}", e);
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        if debug {
                            println!("Invalid UTF8 in message: {:?}", e);
                        }
                    }
                    Err(e) => {
                        if e != zmq::Error::EAGAIN && debug {
                            println!("ZMQ receive error: {:?}", e);
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

        println!("Setting running to true");
        if let Ok(mut running) = self.running.lock() {
            *running = true;
            println!(
                "Running state set to: {} (address: {:p})",
                *running, &self.running
            );
        } else {
            println!("Failed to set running state");
        }
    }

    pub fn stop_streaming(&mut self) {
        println!("Setting running to false");
        if let Ok(mut running) = self.running.lock() {
            *running = false;
            println!(
                "Running state set to: {} (address: {:p})",
                *running, &self.running
            );
        }

        // Kill all streaming processes
        if let Ok(mut processes) = self.streaming_processes.lock() {
            for process in processes.iter_mut() {
                let _ = process.kill();
            }
            processes.clear();
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
                        println!("Python output: {}", line);
                    }
                }
            });
        }

        // Store the process
        if let Ok(mut processes) = self.streaming_processes.lock() {
            processes.push(child);
        }
    }
}

pub fn update_streams(stream_manager: ResMut<StreamManager>) {
    if !*stream_manager.running.lock().unwrap() {
        return;
    }

    if stream_manager.debug {
        println!("Checking for new stream data...");
    }

    while let Ok(data) = stream_manager.receiver.try_recv() {
        if stream_manager.debug {
            println!("Received data for stream: {}", data.stream_id);
        }

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
                        println!(
                            "Warning: Some flight data fields were missing and defaulted to 0.0"
                        );
                    }
                }
                _ => {
                    if stream_manager.debug {
                        println!("Unknown stream_id: {}", data.stream_id);
                    }
                }
            }
        }
    }
}
