import pandas as pd
import json
import sys
import os
from pathlib import Path

def replace_file_name(path, new_file_name):
    directory = os.path.dirname(path)
    return os.path.join(directory, new_file_name)

def convert_to_serializable(val):
    if pd.isna(val):
        return ""
    if isinstance(val, float):
        return str(val)
    return str(val)

script_path = os.path.abspath(__file__)
print(f"DEBUG: Script location: {script_path}", file=sys.stderr, flush=True)

file_path = replace_file_name(script_path, "penguins.csv")

print(f"DEBUG: Looking for file: {os.path.abspath(file_path)}", file=sys.stderr, flush=True)

try:
    # Check if file exists first
    if not Path(file_path).exists():
        print(f"DEBUG: File not found at: {os.path.abspath(file_path)}", file=sys.stderr, flush=True)
        print(json.dumps({
            "error": f"File not found: {file_path}"
        }))
        sys.exit(1)
        
    # Read the CSV file
    df = pd.read_csv(file_path)
    
    # Convert DataFrame to JSON format
    json_data = {
        "columns": df.columns.tolist(),
        "data": [[convert_to_serializable(val) for val in row] for row in df.values.tolist()]
    }
    
    print(json.dumps(json_data))
except Exception as e:
    print(f"DEBUG: Error occurred: {str(e)}", file=sys.stderr, flush=True)
    print(json.dumps({
        "error": str(e)
    }))


