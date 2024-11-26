# arx: A Fast, Mountable File Archive

Arx is a high-performance file archive format built upon the [Jubako container format](https://github.com/jubako/jubako).
It offers a compelling alternative to traditional archive formats like zip and tar, providing significant speed advantages,
especially for large archives and random access operations. Arx archives can even be mounted as read-only filesystems.

## Key Features

* **Fast Creation and Extraction:**  Arx leverages optimized compression algorithms and a structured data layout for significantly faster archive creation and extraction times compared to traditional methods, particularly for larger datasets.
* **Random Access:**  Access individual files within the archive without needing to decompress the entire archive.  This is particularly beneficial for large archives.
* **Read-Only Mounting (Linux and MacoOS):** Mount Arx archives as read-only filesystems using FUSE, allowing you to directly access and work with files within the archive without decompression.
* **Versatile Compression:** Supports various compression algorithms, including zstd (default), lz4, and lzma, allowing you to choose the best option for your data and performance needs.
* **Comprehensive CLI Tool:** A command-line interface simplifies archive creation, extraction, listing, and mounting.
* **Python Bindings:**  A Python wrapper facilitates integration with Python projects.


## Installation

### Using Cargo

Fist, make sure you have the following dependencies installed:

* fuse3: you need the `fuse3.pc` pkgconfig file, depending on your OS it should come with the development package of the `fuse3` library
  
The easiest way to install `arx` is via Cargo, Rust's package manager:

```bash
cargo install arx
```

### Pre-built Binaries

Pre-built binaries for Windows, macOS, and Linux are available for each release on [GitHub Releases](https://github.com/jubako/arx/releases). Download the appropriate binary for your operating system and add it to your system's `PATH` environment variable.

## Usage Examples

**Create an Archive:**

Create an archive named `my_archive.arx` from the directory `my_directory`:

```bash
arx create -o my_archive.arx -r my_directory
```
The `-r` flag indicates recursive inclusion of subdirectories.  You can omit this for non-recursive creation.

To strip a common prefix from the file paths within the archive, use the `--strip-prefix` option:

```bash
arx create -o my_archive.arx -r --strip-prefix /home/user/documents /home/user/documents/my_directory
```

**Extract an Archive:**

Extract the contents of `my_archive.arx` to the directory `my_output_dir`:

```bash
arx extract my_archive.arx -C my_output_dir
```

The `-C` flag specifies the output directory. If omitted, extraction happens in the current directory.

**List Archive Contents:**

List the files and directories within `my_archive.arx`:

```bash
arx list my_archive.arx
```

For a more machine-readable output suitable for scripting, use the `--stable-output` option:

```bash
arx list --stable-output my_archive.arx
```

**Dump a Single File:**

Dump the contents of a specific file (`my_directory/my_file.txt`) within the archive to standard output:

```bash
arx dump my_archive.arx my_directory/my_file.txt
```

To redirect the output to a file, use redirection:

```bash
arx dump my_archive.arx my_directory/my_file.txt my_file.txt
```

**Mount the Archive (Linux and MacOS):**

Mount `my_archive.arx` to a mount point (requires `libfuse-dev` on Linux and `macfuse` on macOS):

```bash
mkdir mount_point
arx mount my_archive.arx mount_point
```

Unmount using the standard `umount` command. If `mount_point` is not provided, a temporary mount point will be created.
The `arx mount` command runs in the background by default. Use the `--foreground` flag to keep it in the foreground.

**Convert Zip/Tar Archives:**


Convert a zip archive (`my_archive.zip`) or a tar archive (`my_archive.tar.gz`) to an Arx archive:

```bash
zip2arx -o my_archive.arx my_archive.zip
tar2arx -o my_archive.arx my_archive.tar.gz
```

You may need to install `zip2arx` and `tar2arx` tools, the same you have installed `arx` tool.

Remote tar archives can also be converted using `tar2arx`:

```bash
tar2arx -o my_archive.arx https://example.com/my_archive.tar.gz
```

## Performance

The following tables compare the performance of Arx to different archive formats.
Tests were conducted on various datasets (the entire Linux kernel, its drivers directory, and its documentation directory) stored on an SSD.
All tests were run on a tmpfs (archive and extracted files stored in memory).
Mount diff time measures the time to diff the mounted archive with the source directory using `diff -r`.
Mounting of tar and zip archives was performed using the `archivemount` tool.
Arx mount is implemented using the fuse API.
Squashfs was mounted using the kernel; SquashfsFuse was mounted using the fuse API; Only `Mount diff` differs between the two.

"Mount diff" times for tar and zip are significantly longer and may not always be fully measured depending on the dataset and system specifications.

The comparaison script is available at [script/compare_archive.py](https://github.com/jubako/arx/blob/main/script/compare_archive.py)


**Linux doc (Documentation directory only of Linux source code):**

|     Type     |  Creation  |   Size   |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | -------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 150ms963μs | 11.10 MB | 038ms395μs | 004ms051μs | 299ms764μs | 005ms618μs |
|           FS | 150ms639μs | 38.45 MB | 106ms821μs | 006ms962μs | 077ms414μs |      498μs |
|     Squashfs | 103ms076μs | 10.60 MB | 098ms787μs | 005ms365μs | 261ms533μs | 002ms088μs |
| SquashfsFuse | 097ms863μs | 10.60 MB |          - |          - | 748ms597μs |          - |
|          Tar | 141ms079μs |  9.68 MB | 065ms744μs | 041ms015μs |     02m41s | 042ms143μs |
|          Zip |   01s083ms | 15.22 MB | 388ms720μs | 037ms044μs |     03m06s | 014ms088μs |

**Ratio `<Archive> time / Arx time` (A ratio > 100% means Arx is better):**

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ---- |
|           FS |     100% | 346% |    278% |    172% |        26% |   9% |
|     Squashfs |      68% |  95% |    257% |    132% |        87% |  37% |
| SquashfsFuse |      65% |  95% |       - |       - |       250% |    - |
|          Tar |      93% |  87% |    171% |   1012% |     53997% | 750% |
|          Zip |     718% | 137% |   1012% |    914% |     62350% | 251% |


**Linux Driver (Driver directory only of Linux source code):**

|     Type     |  Creation  |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx |   01s060ms |  98.23 MB | 241ms699μs | 009ms516μs |   01s290ms | 007ms193μs |
|           FS | 778ms095μs | 799.02 MB | 523ms191μs | 021ms578μs | 467ms559μs |      495μs |
|     Squashfs | 829ms886μs | 121.70 MB | 435ms851μs | 012ms289μs |   01s629ms | 002ms190μs |
| SquashfsFuse | 829ms237μs | 121.70 MB |          - |          - |   03s823ms |          - |
|          Tar | 911ms042μs |  97.96 MB | 515ms178μs | 472ms060μs |          - | 504ms231μs |
|          Zip |   20s498ms | 141.91 MB |   03s665ms | 098ms194μs |          - | 034ms481μs |

**Ratio `<Archive> time / Arx time` (A ratio > 100% means Arx is better):**

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      73% | 813% |    216% |    227% |        36% |    7% |
|     Squashfs |      78% | 124% |    180% |    129% |       126% |   30% |
| SquashfsFuse |      78% | 124% |       - |       - |       296% |     - |
|          Tar |      86% | 100% |    213% |   4961% |          - | 7010% |
|          Zip |    1932% | 144% |   1516% |   1032% |          - |  479% |


**Linux Source Code (Entire Linux source code):**

|     Type     | Creation |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | -------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 02s104ms | 170.97 MB | 435ms846μs | 022ms238μs |   02s829ms | 010ms613μs |
|           FS | 01s605ms |   1.12 GB |   01s046ms | 043ms358μs | 943ms546μs |      493μs |
|     Squashfs | 01s430ms | 201.43 MB | 725ms532μs | 024ms050μs |   03s272ms | 002ms374μs |
| SquashfsFuse | 01s417ms | 201.43 MB |          - |          - |   13s864ms |          - |
|          Tar | 01s479ms | 168.77 MB | 938ms758μs | 799ms550μs |          - | 802ms427μs |
|          Zip | 31s810ms | 252.96 MB |   06s260ms | 256ms137μs |          - | 045ms722μs |

**Ratio `<Archive> time / Arx time` (A ratio > 100% means Arx is better):**

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      76% | 674% |    240% |    195% |        33% |    5% |
|     Squashfs |      68% | 118% |    166% |    108% |       116% |   22% |
| SquashfsFuse |      67% | 118% |       - |       - |       490% |     - |
|          Tar |      70% |  99% |    215% |   3595% |          - | 7561% |
|          Zip |    1511% | 148% |   1436% |   1152% |          - |  431% |

**Kernel Compilation Time (Time needed to compile the whole kernel with default configuration `-j8`):**

| Type | Compilation |
| ---- | ----------- |
|  Arx |         40m |
|   FS |         32m |


Arx archives are slightly larger (about 1%) than tar.zst archives but 15% smaller than squashfs. Creation and full extraction times are comparable to other formats, but listing files and accessing individual files from the archive are much faster using arx or squashfs. Access time is almost constant independently of the archive size, unlike tar, where access time increases significantly with archive size. Mounting an arx archive makes the archive usable without extraction.


## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Sponsoring

I ([@mgautierfr](https://github.com/mgautierfr)) am a freelance developer. All jubako projects are created in my free time, which competes with my paid work.
If you want me to be able to spend more time on Jubako projects, please consider [sponsoring me](https://github.com/sponsors/jubako).
You can also donate on [liberapay](https://liberapay.com/jubako/donate) or [buy me a coffee](https://buymeacoffee.com/jubako).

## License

This project is licensed under the MIT License - see the [LICENSE-MIT](LICENSE-MIT) file for details.
