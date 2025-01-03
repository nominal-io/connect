import zmq
import time
import numpy as np
import sys
import json
import polars as pl
import os

# Add scale factor constant
LAT_LON_SCALE_FACTOR = 10_000  # Scale factor for lat/lon coordinates
ALTITUDE_SCALE_FACTOR = 1 / 10  # Scale factor for altitude
ALTITUDE_OFFSET = 1.0  # Offset for altitude


def stream_data():
    print("Starting flight replay stream", flush=True)

    # Get the directory where this script is located
    script_dir = os.path.dirname(os.path.abspath(__file__))
    # Construct the full path to the CSV file
    csv_path = os.path.join(script_dir, "dji_ocean_flight_filtered.csv")

    # Read initial app state from stdin
    app_state = json.loads(sys.stdin.read())
    print(f"Received initial app state: {app_state}", flush=True)

    # Read the CSV file using the full path
    df = pl.read_csv(csv_path)

    # Calculate relative coordinates by subtracting the first position
    initial_lat = df["OSD.latitude"][0]
    initial_lon = df["OSD.longitude"][0]

    # Calculate relative positions using polars expressions
    df = df.with_columns(
        [
            (pl.col("OSD.latitude") - initial_lat).alias("rel_lat"),
            (pl.col("OSD.longitude") - initial_lon).alias("rel_lon"),
        ]
    )

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
                "timestamp": float(row["timestamps_ns"]) / 1e9,  # Convert ns to seconds
                "rel_lat": float(row["rel_lat"]) * LAT_LON_SCALE_FACTOR,
                "rel_lon": float(row["rel_lon"]) * LAT_LON_SCALE_FACTOR,
                "altitude": (float(row["OSD.height [ft]"]) * ALTITUDE_SCALE_FACTOR)
                + ALTITUDE_OFFSET,
                "pitch": float(row["OSD.pitch"]),
                "roll": float(row["OSD.roll"]),
                "yaw": float(row["OSD.yaw"]),
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
    print("Exiting with an error for testing")
    raise Exception("Test error")
    # stream_data()
