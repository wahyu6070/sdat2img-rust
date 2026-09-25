# sdat2img-rust

A Rust port of [sdat2img](https://github.com/xpirt/sdat2img) by xpirt, luxi78 and howellzhu. It converts an Android sparse data image (`*.new.dat` + `*.transfer.list`) into a raw filesystem image (`*.img`).

- It has no external dependencies and uses only `std`.
- Its output is byte-for-byte identical to `sdat2img.py` 1.2, and it produces the same sparse files.
- It builds to a single fast binary, so you don't need Python.

## Download

Prebuilt binaries are on the [Releases](https://github.com/wahyu6070/sdat2img-rust/releases) page. Each zip has one folder per architecture:

| File                   | Architectures                                                  |
|------------------------|----------------------------------------------------------------|
| `sdat2img-linux.zip`   | `arm`, `arm64`, `x86`, `x86_64` (static, no dependencies)      |
| `sdat2img-android.zip` | `arm`, `arm64`, `x86`, `x86_64` (Android 5.0+ / API 21)        |
| `sdat2img-windows.zip` | `x86`, `x86_64`, `arm64` (Windows 10+)                         |

32-bit ARM Windows has no build because Rust has no usable target for it.

To run the Android binary on a device (with adb or Termux):

```sh
adb push arm64/sdat2img /data/local/tmp/
adb shell chmod 755 /data/local/tmp/sdat2img
adb shell /data/local/tmp/sdat2img system.transfer.list system.new.dat system.img
```

## Build

You need [Rust](https://rustup.rs) 1.88 or newer.

```sh
cargo build --release
# binary: target/release/sdat2img
```

To install the binary into `~/.cargo/bin`:

```sh
cargo install --path .
```

### Cross-compiling

`build.sh` builds every release target and packages them into `dist/sdat2img-<os>.zip` plus `dist/SHA256SUMS`:

```sh
./build.sh            # all targets
./build.sh linux      # only Linux (also: android, windows)
./build.sh linux-arm64 windows-x86_64
./build.sh --list     # show all targets
```

It needs:

- [zig](https://ziglang.org/download/) and [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) (`cargo install --locked cargo-zigbuild`) for the Linux and Windows targets.
- The [Android NDK](https://developer.android.com/ndk/downloads) for the Android targets. Set `ANDROID_NDK_HOME` or extract it into `~/Android`.
- `zip`.

The script installs missing Rust targets on its own.

## Usage

```
sdat2img <transfer_list> <system_new_file> [system_img]

    <transfer_list>:   transfer list file
    <system_new_file>: system new dat file
    [system_img]:      output system image (default: system.img)
```

Example:

```sh
sdat2img system.transfer.list system.new.dat system.img
```

If your file is brotli-compressed (`.new.dat.br`), decompress it first:

```sh
brotli -d system.new.dat.br
```

## Supported transfer list versions

| Version | Android                 |
|---------|-------------------------|
| 1       | Lollipop 5.0            |
| 2       | Lollipop 5.1            |
| 3       | Marshmallow 6.x         |
| 4       | Nougat 7.x / Oreo 8.x+  |

The tool handles only the `erase`, `new` and `zero` commands, which are the ones full OTAs use. It rejects transfer lists from incremental OTAs (`move`, `bsdiff`, `imgdiff`, `stash` and similar) with an error.

## Differences from the Python version

- If `new.dat` is shorter than the transfer list requires, the program stops with an error. The Python version silently produces a corrupt image.
- It reports malformed rangesets (an odd count, or `begin > end`) as errors and skips blank lines, where the Python version crashes.
- Run with no arguments, it prints usage and exits with code 1. It doesn't wait for ENTER.
- Like the Python version, it **overwrites** an existing output file.

## Testing

```sh
cargo test
```

## License

MIT, following the original [xpirt/sdat2img](https://github.com/xpirt/sdat2img) project.
