import zmq
import time
import numpy as np
import sys
import json
import polars as pl

def stream_data():
    print("Starting flight replay stream", flush=True)
    
    # Read initial app state from stdin
    app_state = json.loads(sys.stdin.read())
    print(f"Received initial app state: {app_state}", flush=True)

    # Load the CSV file with polars
    df = pl.read_csv('dji_ocean_flight_filtered.csv')
    
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
        # Stream each row of the dataframe
        for row in df.iter_rows(named=True):
            data = {
                "stream_id": "flight_position",
                "timestamp": float(row['timestamps_ns']) / 1e9,  # Convert ns to seconds
                "rel_lat": float(row['rel_lat']),
                "rel_lon": float(row['rel_lon'])
            }
            print(f"Sending data: {data}", flush=True)
            socket.send_string(json.dumps(data))
            time.sleep(0.01)  # Add a small delay
    except Exception as e:
        print(f"Error in stream_data: {e}", flush=True)
    finally:
        print("Shutting down...", flush=True)
        socket.close()
        context.term()

if __name__ == "__main__":
    stream_data()