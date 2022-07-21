# Sbyte
An in-console byte editor. Work in progress, but fairly stable for basic work.<br/>
[![Crates.io](https://img.shields.io/crates/v/sbyte?style=flat-square)](https://crates.io/crates/sbyte)
[![Crates.io](https://img.shields.io/crates/d/sbyte?style=flat-square)](https://crates.io/crates/sbyte)
[![GitHub](https://img.shields.io/github/license/quintinfsmith/sbyte?style=flat-square)](https://github.com/quintinfsmith/sbyte/blob/master/LICENSE)


## About
The environment was designed to feel and work as much like VIM as is reasonable for a byte editor.
(hjkl directional controls, numerical register, a command line with its own pseudo-language as well as modes [visual, insert, append, replace])

## Installation & Usage
### From crates.io
```
cargo install sbyte
sbyte <filename>
```

### From source:
```
cargo install --git git://burnsomni.net/sbyte
sbyte <filename>
```

See sbyterc for basic controls.
