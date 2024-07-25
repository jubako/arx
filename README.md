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
|          Arx | 151ms148μs | 11.10 MB | 038ms527μs | 003ms381μs | 250ms776μs | 004ms909μs |
|           FS | 152ms331μs | 38.45 MB | 105ms728μs | 007ms509μs | 078ms462μs |      496μs |
|     Squashfs | 099ms716μs | 10.60 MB | 085ms815μs | 004ms637μs | 272ms953μs | 002ms106μs |
| SquashfsFuse | 097ms174μs | 10.60 MB |          - |          - | 775ms089μs |          - |
|          Tar | 129ms599μs |  9.68 MB | 064ms772μs | 041ms099μs |     02m48s | 043ms755μs |
|          Zip |   01s111ms | 15.22 MB | 370ms369μs | 028ms685μs |     03m09s | 014ms194μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ---- |
|           FS |     101% | 346% |    274% |    222% |        31% |  10% |
|     Squashfs |      66% |  95% |    223% |    137% |       109% |  43% |
| SquashfsFuse |      64% |  95% |       - |       - |       309% |    - |
|          Tar |      86% |  87% |    168% |   1216% |     67104% | 891% |
|          Zip |     735% | 137% |    961% |    848% |     75548% | 289% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx |   01s085ms |  98.23 MB | 285ms486μs | 009ms975μs |   01s298ms | 005ms219μs |
|           FS | 800ms259μs | 799.02 MB | 519ms484μs | 018ms222μs | 473ms239μs |      499μs |
|     Squashfs | 829ms672μs | 121.70 MB | 420ms767μs | 011ms155μs |   01s591ms | 002ms201μs |
| SquashfsFuse | 827ms695μs | 121.70 MB |          - |          - |   03s808ms |          - |
|          Tar | 906ms266μs |  97.96 MB | 509ms529μs | 467ms522μs |          - | 514ms628μs |
|          Zip |   20s692ms | 141.91 MB |   03s730ms | 096ms525μs |          - | 034ms796μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      74% | 813% |    182% |    183% |        36% |   10% |
|     Squashfs |      76% | 124% |    147% |    112% |       123% |   42% |
| SquashfsFuse |      76% | 124% |       - |       - |       293% |     - |
|          Tar |      83% | 100% |    178% |   4687% |          - | 9861% |
|          Zip |    1906% | 144% |   1307% |    968% |          - |  667% |


Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | -------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 02s291ms | 170.97 MB | 482ms533μs | 023ms408μs |   02s825ms | 005ms316μs |
|           FS | 01s627ms |   1.12 GB |   01s101ms | 047ms314μs | 956ms438μs |      480μs |
|     Squashfs | 01s425ms | 201.43 MB | 743ms588μs | 027ms390μs |   03s274ms | 002ms379μs |
| SquashfsFuse | 01s441ms | 201.43 MB |          - |          - |   14s433ms |          - |
|          Tar | 01s560ms | 168.77 MB |   01s103ms | 845ms941μs |          - | 824ms696μs |
|          Zip | 32s022ms | 252.96 MB |   06s499ms | 259ms919μs |          - | 045ms474μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff |  Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ------ |
|           FS |      71% | 674% |    228% |    202% |        34% |     9% |
|     Squashfs |      62% | 118% |    154% |    117% |       116% |    45% |
| SquashfsFuse |      63% | 118% |       - |       - |       511% |      - |
|          Tar |      68% |  99% |    229% |   3614% |          - | 15513% |
|          Zip |    1397% | 148% |   1347% |   1110% |          - |   855% |

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
