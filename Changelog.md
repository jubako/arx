# Arx 0.2

This release is a huge release!!
Most of the changes come from the Jubako 0.2.0 release.

## Create a arx library

Arx is now split in a library and a tool part.

The arx library (libarx) can now be used by other projects.

## New `automount` binary.

The `automount` binary is useless in itself. But it can be concatenated with an Arx archive to
create an automount archive.


## Store unix properties in Arx

Now arx format stores file properties. The properties are:

- owner
- group
- mode
- mtime

The size is also store in the entry, which allow libarx to give the file size without searching
for content in the contentPack (this somehow prevents use to use variant contentPacks).

## Do deduplication at file level.

At creation, arx maintain a cache of hash of already added content.
If a file to add has the same hash that a previously added content, we will reuse the previous ContentAddress.

## Various improvement of the command line API

- `arx list` now have a stable output,
- `arx extract` and `arx create` can now take a file listing the files to extract/add
- `arx extract` has a `--progress` option
- `arx create` has an option to strip prefix from the add file name
- `arx create` has an option to change current directory before creating the archive
- `arx create` has options to specify if we want one file (containing all packs) or one file per packs
- `arx create` has an option to configure the compression used

## Add zip2arx and tar2arx

## Port to Windows and MacOs

While arx compile on Windows and MacOs, it is not completely tested.
`mount` feature is not available on Windows.

## Various and numerous performance improvements
