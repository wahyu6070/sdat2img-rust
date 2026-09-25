# sdat2img-rust

Port Rust dari [sdat2img](https://github.com/xpirt/sdat2img) (xpirt, luxi78, howellzhu): alat untuk mengubah Android sparse data image (`*.new.dat` + `*.transfer.list`) menjadi image filesystem mentah (`*.img`).

- Tanpa dependensi eksternal (hanya `std`).
- Hasilnya identik byte-per-byte dengan `sdat2img.py` versi 1.2, termasuk file sparse-nya.
- Satu binary statis dan cepat. Anda tidak perlu Python.

## Build

Anda perlu [Rust](https://rustup.rs) versi 1.88 atau lebih baru.

```sh
cargo build --release
# binary: target/release/sdat2img
```

Untuk memasang binary ke `~/.cargo/bin`:

```sh
cargo install --path .
```

## Penggunaan

```
sdat2img <transfer_list> <system_new_file> [system_img]

    <transfer_list>:   file transfer list
    <system_new_file>: file system new dat
    [system_img]:      image output (default: system.img)
```

Contoh:

```sh
sdat2img system.transfer.list system.new.dat system.img
```

Jika file Anda berformat `.new.dat.br` (brotli), dekompres dulu:

```sh
brotli -d system.new.dat.br
```

## Versi transfer list yang didukung

| Versi | Android                 |
|-------|-------------------------|
| 1     | Lollipop 5.0            |
| 2     | Lollipop 5.1            |
| 3     | Marshmallow 6.x         |
| 4     | Nougat 7.x / Oreo 8.x+  |

Tool ini hanya menangani perintah `erase`, `new`, dan `zero`, yaitu OTA penuh (*full OTA*). Transfer list dari OTA inkremental (`move`, `bsdiff`, `imgdiff`, `stash`, dan sejenisnya) akan ditolak dengan pesan error.

## Perbedaan dengan versi Python

- Jika `new.dat` lebih pendek dari yang dibutuhkan transfer list, program berhenti dengan error. Versi Python diam-diam menghasilkan image yang rusak.
- Rangeset yang tidak valid (jumlah angka ganjil, `begin > end`) dan baris kosong ditangani dengan rapi, tanpa crash.
- Program tidak menunggu tombol ENTER saat dijalankan tanpa argumen. Ia hanya menampilkan cara pakai lalu keluar dengan kode 1.
- Sama seperti versi Python, file output yang sudah ada akan **ditimpa**.

## Pengujian

```sh
cargo test
```

## Lisensi

MIT, mengikuti proyek aslinya [xpirt/sdat2img](https://github.com/xpirt/sdat2img).
