import sys
import json
import argparse

def echo_one(state):
    print("Echo One!")
    print(f"App state: {state}")

def echo_two(state):
    print("Echo Two!")
    print(f"App state: {state}")

if __name__ == "__main__":
    # Parse arguments to check for function-specific execution
    parser = argparse.ArgumentParser()
    parser.add_argument("--function", help="Function to execute")
    args = parser.parse_args()
    
    # Read state from stdin
    state = json.loads(sys.stdin.read())
    
    if args.function:
        # Execute specific function if requested
        if args.function in globals():
            globals()[args.function](state)
        else:
            print(f"Error: Function '{args.function}' not found")
    else:
        # Execute all functions if no specific function requested
        echo_one(state)
        echo_two(state)