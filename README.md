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
cargo install --git https://burnsomni.net/git/sbyte
sbyte <filename>
```

See sbyterc for basic controls.

## Hex, Binary & Decimal Views
Sbyte may be a hex editor, but it's also a bin editor and dec editor. Switch between them on-the-fly

## Regex modifications
Regex is supported in searches, however some modifications have been made to make it more useful in the context of all bytes rather than just the human-readable ones.

### Byte Wildcarding
Use a `.` to indicate a wildcard within a byte.

#### Examples
This will find all bytes from \x90 to \x9F:
```
find \x9.
```

This can also be done in binary:
```
find \b1001....
```
and doesn't need to be sequential
```
find \b100100.0
```
will match \x90 & \x92


