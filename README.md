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
|          Arx | 145ms703μs | 11.10 MB | 040ms768μs | 003ms140μs | 259ms330μs | 005ms008μs |
|           FS | 150ms698μs | 38.45 MB | 099ms800μs | 007ms613μs | 086ms694μs |      492μs |
|     Squashfs | 097ms521μs | 10.60 MB | 080ms002μs | 004ms677μs | 270ms579μs | 002ms088μs |
| SquashfsFuse | 101ms727μs | 10.60 MB |          - |          - | 767ms645μs |          - |
|          Tar | 124ms293μs |  9.68 MB | 062ms599μs | 040ms320μs |     02m44s | 043ms025μs |
|          Zip |   01s069ms | 15.22 MB | 351ms623μs | 028ms918μs |     03m01s | 014ms063μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ---- |
|           FS |     103% | 346% |    245% |    242% |        33% |  10% |
|     Squashfs |      67% |  95% |    196% |    149% |       104% |  42% |
| SquashfsFuse |      70% |  95% |       - |       - |       296% |    - |
|          Tar |      85% |  87% |    154% |   1284% |     63282% | 859% |
|          Zip |     734% | 137% |    862% |    921% |     70111% | 281% |


Linux Driver
------------

Driver directory only of linux source code:

|     Type     |  Creation  |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | ---------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx |   01s069ms |  98.23 MB | 277ms263μs | 012ms094μs |   01s276ms | 005ms182μs |
|           FS | 752ms209μs | 799.02 MB | 501ms121μs | 019ms960μs | 454ms756μs |      497μs |
|     Squashfs |   01s607ms | 121.70 MB | 442ms331μs | 012ms692μs |   01s580ms | 002ms206μs |
| SquashfsFuse | 829ms085μs | 121.70 MB |          - |          - |   03s771ms |          - |
|          Tar | 966ms268μs |  97.96 MB | 510ms003μs | 466ms650μs |          - | 512ms660μs |
|          Zip |   20s330ms | 141.91 MB |   03s541ms | 096ms414μs |          - | 034ms747μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff | Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ----- |
|           FS |      70% | 813% |    181% |    165% |        36% |   10% |
|     Squashfs |     150% | 124% |    160% |    105% |       124% |   43% |
| SquashfsFuse |      78% | 124% |       - |       - |       296% |     - |
|          Tar |      90% | 100% |    184% |   3859% |          - | 9893% |
|          Zip |    1901% | 144% |   1277% |    797% |          - |  671% |



Linux Source Code
-----------------

|     Type     | Creation |   Size    |  Extract   |  Listing   | Mount diff |    Dump    |
| ------------ | -------- | --------- | ---------- | ---------- | ---------- | ---------- |
|          Arx | 02s319ms | 170.97 MB | 503ms943μs | 024ms115μs |   02s809ms | 005ms326μs |
|           FS | 01s560ms |   1.12 GB |   01s034ms | 040ms061μs | 923ms114μs |      501μs |
|     Squashfs | 01s446ms | 201.43 MB | 750ms447μs | 023ms492μs |   03s392ms | 002ms397μs |
| SquashfsFuse | 01s494ms | 201.43 MB |          - |          - |   14s594ms |          - |
|          Tar | 01s555ms | 168.77 MB |   01s071ms | 847ms997μs |          - | 822ms503μs |
|          Zip | 31s694ms | 252.96 MB |   06s568ms | 258ms823μs |          - | 045ms512μs |

This is the ratio <Archive> time / Arx time.
A ratio greater than 100% means Arx is better.

|     Type     | Creation | Size | Extract | Listing | Mount diff |  Dump  |
| ------------ | -------- | ---- | ------- | ------- | ---------- | ------ |
|           FS |      67% | 674% |    205% |    166% |        33% |     9% |
|     Squashfs |      62% | 118% |    149% |     97% |       121% |    45% |
| SquashfsFuse |      64% | 118% |       - |       - |       519% |      - |
|          Tar |      67% |  99% |    213% |   3516% |          - | 15443% |
|          Zip |    1366% | 148% |   1303% |   1073% |          - |   855% |

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
