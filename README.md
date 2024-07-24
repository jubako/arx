What is arx
===========

Arx is a file archive format based on the
[jubako container format](https://github.com/jubako/jubako).

It allow you to create, read, extract file archive (as zip or tar does).

Arx (and Jubako) is in active development.
While it works pretty well, I do not recommand to use it to do backups.
However, you can use it to transfer data or explore archives.


How it works
============


Jubako is a versatile container format, allowing to store data, compressed or
not in a structured way. It main advantage (apart from its versability) is
that is designed to allow quick retrieval of data fro the archive without
needing to uncompress the whole archive.

Arx use the jubako format and create arx archive which:
- Store file's data compressed.
- Store files using a directory/tree structure.
- Can do random access on the arx archive to get a specific files
- Allow to mount the archive to explore and use (read only) the files in the
  archive without decompressing it.

Install arx
===========

Binaries for Windows, MacOS and Linux are available for [every release](https://github.com/jubako/arx/releases).
You can also install arx using Cargo:

```
cargo install arx
```

Use arx
=======

Create an archive
-----------------

Creating an archive is simple :


```
arx create -o my_archive.arx -r my_directory
```

It will one file : `my_archive.arx`, which will contains the `my_directory` directory.


Extract an archive
------------------

Extracting (decompressing) an archive is done with :

```
arx extract my_archive.arx -C my_out_dir
```


Listing the content of an archive
---------------------------------

You can list the content of (the list of files in) the archive with :

```
arx list my_archive.arx
```

And if you want to access to the content of only one file :

```
arx dump my_archive.arx my_directory/path/to/my_file > my_file
# or
arx dump my_archive.arx my_directory/path/to/my_file -o my_file
```

Mounting the archive
--------------------

On linux, you can mount the archive using fuse.

```
mkdir mount_point
arx mount my_archive.arx mount_point
```

If you don't provide a `mount_point`, arx will create a temporary one for you

```
arx mount my_archive.arx # Will create my_archive.arx.xxxxxx
```

`arx` will be running until you unmount `mount_point`.

Converting a zip archive to an arx archive
------------------------------------------

```
zip2arx -o my_archive.arx my_zip_archive.zip
```

Converting a tar archive to an arx archive
------------------------------------------

```
tar2arx -o my_archive.arx my_tar_archive.tar.gz
```

or

```
tar2arx -o my_archive.arx https://example.com/my_tar_archive.tar.gz
```


Performance
===========

The following compare the performance of Arx to different archive formats.

- Arx, Tar, Squasfs is compressed the content using zstd, level 5.
- Zip is compressed using level 9
- Fs is FileSystem (no archive). Archive creation and extraction is simulated with `cp -a`.

Tests has been done on different data sets :
- the whole linux kernel (linux-5.19)
- the drivers directory in linux kernel
- the document directory in the linux kernel

Source directory is stored on a sdd. All test are run on a tmpfs (archive and
extracted files are stored in memory).

Mount diff time is the time to diff the mounted archive with the source directory
```
arx mount archive.arx mount_point &
time diff -r mount_point/linux-5.19 linux-5.19
umount mount_point
```
Mounting the tar and zip archive is made with `archivemount` tool.
Squashfs is mounted using kernel. SquashfsFuse is mounted using fuse API.
Arx mount is implemented using fuse API.

Linux doc
---------

Documentation directory only of linux source code:

|     Type     |  Creation  |   Size   |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | -------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 148ms429μs | 11.10 MB | 039ms054μs | 003ms091μs | 239ms508μs | 004ms905μs |
|           FS | 153ms613μs | 38.45 MB | 104ms119μs | 007ms966μs | 087ms292μs |      506μs |
|     Squashfs | 099ms679μs | 10.60 MB | 099ms296μs | 004ms905μs | 262ms587μs | 002ms114μs |
| SquashfsFuse | 102ms805μs | 10.60 MB |          - |          - | 759ms347μs |          - |
|          Tar | 133ms468μs |  9.68 MB | 064ms088μs | 041ms584μs |     02m43s | 042ms650μs |
|          Zip |   01s097ms | 15.22 MB | 363ms020μs | 033ms528μs |     03m05s | 015ms047μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ---- |
|           FS |     103% | 346% |    267% |    258% |        36% |  10% |
|     Squashfs |      67% |  95% |    254% |    159% |       110% |  43% |
| SquashfsFuse |      69% |  95% |       - |       - |       317% |    - |
|          Tar |      90% |  87% |    164% |   1345% |     68164% | 870% |
|          Zip |     739% | 137% |    930% |   1085% |     77640% | 307% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx |   01s106ms |  98.23 MB | 281ms968μs | 011ms492μs |   01s298ms | 005ms199μs |
|           FS | 786ms361μs | 799.02 MB | 530ms671μs | 020ms526μs | 475ms749μs |      504μs |
|     Squashfs | 830ms269μs | 121.70 MB | 418ms329μs | 010ms964μs |   01s600ms | 002ms206μs |
| SquashfsFuse | 831ms133μs | 121.70 MB |          - |          - |   03s839ms |          - |
|          Tar | 901ms444μs |  97.96 MB | 505ms034μs | 470ms764μs |          - | 515ms216μs |
|          Zip |   20s400ms | 141.91 MB |   03s689ms | 100ms486μs |          - | 035ms018μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      71% | 813% |    188% |    179% |        37% |   10% |
|     Squashfs |      75% | 124% |    148% |     95% |       123% |   42% |
| SquashfsFuse |      75% | 124% |       - |       - |       296% |     - |
|          Tar |      82% | 100% |    179% |   4096% |          - | 9910% |
|          Zip |    1844% | 144% |   1309% |    874% |          - |  674% |



Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | -------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 02s270ms | 170.97 MB | 500ms671μs | 021ms980μs |   02s863ms | 005ms345μs |
|           FS | 01s634ms |   1.12 GB |   01s091ms | 041ms677μs | 963ms072μs |      497μs |
|     Squashfs | 01s434ms | 201.43 MB | 758ms057μs | 025ms566μs |   03s295ms | 002ms374μs |
| SquashfsFuse | 01s437ms | 201.43 MB |          - |          - |   14s481ms |          - |
|          Tar | 01s551ms | 168.77 MB |   01s096ms | 880ms099μs |          - | 825ms473μs |
|          Zip | 32s327ms | 252.96 MB |   06s433ms | 264ms517μs |          - | 045ms976μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff |  Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ------ |
|           FS |      72% | 674% |    218% |    190% |        34% |     9% |
|     Squashfs |      63% | 118% |    151% |    116% |       115% |    44% |
| SquashfsFuse |      63% | 118% |       - |       - |       506% |      - |
|          Tar |      68% |  99% |    219% |   4004% |          - | 15444% |
|          Zip |    1424% | 148% |   1285% |   1203% |          - |   860% |

The kernel compilation is the time needed to compile the whole kernel with the
default configuration (-j8). For arx, we are compiling the kernel using the
source in the archive mounted in mount_point.

Kernel compilation is made is "real" condition. Source or arx archive are stored on ssd.


| Type | Compilation |
| ---- | ----------- |
|  Arx |         40m |
|   FS |         32m |


Arx archive are a bit bigger (about 1%) than tar.zst archive but 15% smaller that squashfr.
Creation and full extraction time are a bit longer for arx but times are comparable.

Listing files ar accessing individual files from the archive is far more rapid using arx or squash.
Access time is almost constant indpendently of the size of the archive.
For tar however, time to access individual file is greatly increasing when the
archive size is increasing.

Mounting a arx archive make the archive usable without extracting it.
A simple `diff -r` takes 4 more time than a plain diff between two directories but
it is a particular use case (access all files "sequentially" and only once).

But for linux documentation arx is 444 time quicker than tar (several hours).
The bigger the tar archive is the bigger is this ratio. I haven't try to do a
mount-diff for the full kernel.

For kernel compilation, the overhead is about 25%.
But on the opposite side, you can compile the kernel without storing 1.3GB of
source on your hard drive.
