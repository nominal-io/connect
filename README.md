# connect
Connect your hardware to Nominal

🚧 Prototype - changing daily 🚧

## Get started

1. Clone this repo
2. [Install Rust](https://www.rust-lang.org/tools/install) (try `rustup -V` in your terminal to check)
3. [Install UV](https://docs.astral.sh/uv/getting-started/installation/)
3. In the `connect` folder, create and activate a virtual environment:

```sh
python3 -m venv .venv
source .venv/bin/activate
```

4. Install ZMQ & numpy in this environment: `pip3 install pyzmq numpy`
5. Run `cargo run` - an app should appear with the default config loaded

## Usage

### Running Python scripts

Python scripts that connect to your hardware are set in a config.toml file:

```toml
[[scripts]]
name = "Print random"
path = "scripts/rand.py"
type = "discrete"

[[scripts]]
name = "Sine Wave"
path = "stream_example.py"
type = "streaming"

[[scripts]]
name = "Echo inputs"
path = "echo.py"
type = "discrete"
functions = [
    { name = "echo_one", display = "Field 1" },
    { name = "echo_two", display = "Field 2" }
]
```

In the future, additional languages may be supported (C, Rust, MATLAB, etc).

- `discrete` scripts are run once
- `streaming` scripts are run continuously and can push to an IPC (ZMQ) channel
- To execute individual functions within a script file, set the `functions` parameter.

Scripts can receive an app state - a JSON payload of return values from other scripts and the UI state of various input widgets (sliders, text fields, etc). See `echo.py` in `recipes/kitchen_sink` for a simple example of extracting app state in Python.

### App layout

App layout is also configured in the TOML file. For example, to add a slider:

```toml
[[layout.sliders]]
id = "frequency"
label = "Frequency"
min = 0.0
max = 10.0
default = 1.0
```

## Screenshot

<img width="1271" alt="image" src="https://github.com/user-attachments/assets/24c59730-d69a-4ad1-b8a4-81c8f5f5527d">





  
