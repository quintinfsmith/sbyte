# Sbyte
An in-console byte editor. Work in progress, but fairly stable for basic work.

## About
The environment was designed to feel and work as much like VIM as is reasonable for a byte editor. 
(hjkl directional controls, numerical register, a command line with its own pseudo-language as well as modes [visual, insert, append, replace])

## Installation & Usage
### From source:
```
cargo clone https://github.com/quintinfsmith/sbyte.git
cd sbyte
cargo run --release <filename>
```

### For Arch:
```
wget https://github.com/quintinfsmith/sbyte/releases/download/v0.1.0/sbyte-0.1.0.tar.gz
tar -xvf sbyte-0.1.0.tar.gz
cd sbyte
makepkg -si
sbyte path/to/file
```

### For Debian:
```
wget https://github.com/quintinfsmith/sbyte/releases/download/v0.1.0/sbyte-0.1.0.deb
dpkg -i sbyte-0.1.0.deb
sbyte path/to/file
```
...or just run the .deb from the filemanager.

rpm packages are in the hopper.

See sbyterc for basic controls.
