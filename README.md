# Sbyte
An in-console byte editor. Work in progress, but fairly stable for basic work.

## About
The environment was designed to feel and work as much like VIM as is reasonable for a byte editor. 
(hjkl directional controls, numerical register, a command line with its own pseudo-language as well as modes [visual, insert, append, replace])

## Installation & Usage
While I build the scripts to package everything,
```
cargo clone https://github.com/quintinfsmith/sbyte.git
cd sbyte
cargo run --release <filename>
```
But arch, deb & rpm packages are in the hopper.

See sbyterc for basic controls
