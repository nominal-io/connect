import zmq
import time
import numpy as np
import sys
import json
import polars as pl
import os

# Add scale factor constant
LAT_LON_SCALE_FACTOR = 10_000  # Scale factor for lat/lon coordinates
ALTITUDE_SCALE_FACTOR = 1/10  # Scale factor for altitude
ALTITUDE_OFFSET = 1.0  # Offset for altitude

def stream_data():
    print("Starting flight replay stream", flush=True)
    
    # Get the directory where this script is located
    script_dir = os.path.dirname(os.path.abspath(__file__))
    # Construct the full path to the CSV file
    csv_path = os.path.join(script_dir, 'dji_ocean_flight_filtered.csv')
    
    # Read initial app state from stdin
    app_state = json.loads(sys.stdin.read())
    print(f"Received initial app state: {app_state}", flush=True)

    # Read the CSV file using the full path
    df = pl.read_csv(csv_path)
    
    # Calculate relative coordinates by subtracting the first position
    initial_lat = df['OSD.latitude'][0]
    initial_lon = df['OSD.longitude'][0]
    
    # Calculate relative positions using polars expressions
    df = df.with_columns([
        (pl.col('OSD.latitude') - initial_lat).alias('rel_lat'),
        (pl.col('OSD.longitude') - initial_lon).alias('rel_lon')
    ])

    context = zmq.Context()
    socket = context.socket(zmq.PUSH)
    print("Creating ZMQ PUSH socket...", flush=True)
    socket.bind("tcp://*:5555")
    print("Bound ZMQ socket to tcp://*:5555", flush=True)

    try:
        while True:  # Add continuous loop
            # Stream each row of the dataframe
            for row in df.iter_rows(named=True):
                timestamp = float(row['timestamps_ns']) / 1e9  # Convert ns to seconds
                
                # Stream flight position data
                flight_data = {
                    "stream_id": "flight_position",
                    "timestamp": timestamp,
                    "rel_lat": float(row['rel_lat']) * LAT_LON_SCALE_FACTOR,
                    "rel_lon": float(row['rel_lon']) * LAT_LON_SCALE_FACTOR,
                    "altitude": (float(row['OSD.height [ft]']) * ALTITUDE_SCALE_FACTOR) + ALTITUDE_OFFSET,
                    "pitch": float(row['OSD.pitch']),
                    "roll": float(row['OSD.roll']),
                    "yaw": float(row['OSD.yaw'])
                }
                socket.send_string(json.dumps(flight_data))

                # Stream yaw data for 2D plot
                yaw_data = {
                    "stream_id": "aircraft_pitch",
                    "timestamp": timestamp,
                    "value": float(row['OSD.pitch'])
                }
                socket.send_string(json.dumps(yaw_data))
                
                # Stream altitude data for 2D plot
                altitude_data = {
                    "stream_id": "aircraft_altitude",
                    "timestamp": timestamp,
                    "value": float(row['OSD.height [ft]'])
                }
                socket.send_string(json.dumps(altitude_data))
                
                time.sleep(0.01)  # Add a small delay
            
            # Optional: Add a small delay between replays
            time.sleep(0.1)
            
    except Exception as e:
        print(f"Error in stream_data: {e}", flush=True)
    finally:
        print("Shutting down...", flush=True)
        socket.close()
        context.term()

if __name__ == "__main__":
    stream_data()