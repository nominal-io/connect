use std::process::{Child};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde::Deserialize;
use std::io::{BufReader, BufRead};
use std::thread;
use bevy::prelude::*;

#[derive(Resource)]
pub struct StreamManager {
    pub streams: Arc<Mutex<HashMap<String, Vec<[f64; 2]>>>>,
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
    pub value: f64,
}

impl StreamManager {
    pub fn new(debug: bool) -> Self {
        let (sender, receiver) = bounded(100);
        let running = Arc::new(Mutex::new(false));
        let running_clone = Arc::clone(&running);
        let debug = debug;
        let sender_clone = sender.clone();

        // Spawn ZMQ listener thread
        thread::spawn(move || {
            if debug { println!("Starting ZMQ listener thread"); }
            let context = zmq::Context::new();
            let subscriber = context.socket(zmq::PULL).unwrap();
            
            if debug { println!("Binding ZMQ socket to tcp://*:5555"); }
            if let Err(e) = subscriber.bind("tcp://*:5555") {
                if debug { println!("Failed to bind: {:?}", e); }
            }
            
            loop {
                let is_running = running_clone.lock()
                    .map(|guard| *guard)
                    .unwrap_or_else(|e| {
                        if debug { println!("Failed to lock running state: {:?}", e); }
                        false
                    });

                if !is_running {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                match subscriber.recv_string(zmq::DONTWAIT) {
                    Ok(Ok(msg)) => {
                        if debug { println!("ZMQ received message: {}", msg); }
                        match serde_json::from_str::<StreamData>(&msg) {
                            Ok(data) => {
                                if debug { println!("Parsed message into data: {:?}", data); }
                                if sender_clone.send(data).is_err() {
                                    if debug { println!("Failed to send data through channel"); }
                                    break;
                                }
                            },
                            Err(e) => if debug { println!("Failed to parse message: {:?}", e); }
                        }
                    },
                    Ok(Err(e)) => if debug { println!("Invalid UTF8 in message: {:?}", e); },
                    Err(e) => if e != zmq::Error::EAGAIN && debug {
                        println!("ZMQ receive error: {:?}", e);
                    },
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
            println!("Running state set to: {} (address: {:p})", *running, &self.running);
        } else {
            println!("Failed to set running state");
        }
    }

    pub fn stop_streaming(&mut self) {
        println!("Setting running to false");
        if let Ok(mut running) = self.running.lock() {
            *running = false;
            println!("Running state set to: {} (address: {:p})", *running, &self.running);
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

    while let Ok(data) = stream_manager.receiver.try_recv() {
        if stream_manager.debug {
            println!("Received new data point - Stream: {}, Time: {}, Value: {}", 
                data.stream_id, data.timestamp, data.value);
        }
        
        if let Ok(mut streams) = stream_manager.streams.lock() {
            let points = streams.entry(data.stream_id.clone()).or_default();
            points.push([data.timestamp, data.value]);
            
            if points.len() > 100 {
                points.remove(0);
            }
            if stream_manager.debug {
                println!("Stream {} now has {} points", data.stream_id, points.len());
            }
        }
    }
}
