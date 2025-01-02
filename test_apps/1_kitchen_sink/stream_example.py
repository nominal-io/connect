import zmq
import time
import numpy as np
import sys
import json

def stream_data():
    print("Starting stream_example.py", flush=True)
    
    # Read initial app state from stdin
    app_state = json.loads(sys.stdin.read())
    print(f"Received initial app state: {app_state}", flush=True)

    # Initialize state variables from initial app state
    frequency = app_state.get('slider_values', {}).get('frequency', 1.0)
    y_offset = app_state.get('slider_values', {}).get('y_axis_offset', 0.0)
    
    print(f"Initial values - frequency: {frequency}, y_offset: {y_offset}", flush=True)

    context = zmq.Context()
    
    # Data streaming socket
    socket = context.socket(zmq.PUSH)
    print("Creating ZMQ PUSH socket...", flush=True)
    socket.bind("tcp://*:5555")
    print("Bound ZMQ socket to tcp://*:5555", flush=True)

    try:
        t = 0
        while True:
            value = np.sin(t * frequency) + y_offset
            
            data = {
                "stream_id": "single_scalar_channel",
                "timestamp": float(t),
                "value": float(value)
            }
            print(f"Sending data: {data}", flush=True)
            socket.send_string(json.dumps(data))
            print(f"Data sent successfully", flush=True)
            t += 0.1
            time.sleep(0.01)  # Add a small delay
    except Exception as e:
        print(f"Error in stream_data: {e}", flush=True)
    finally:
        print("Shutting down...", flush=True)
        socket.close()
        context.term()

if __name__ == "__main__":
    stream_data()