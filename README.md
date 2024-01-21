syosetu-dump
==============

![](https://github.com/DoumanAsh/syosetu-dump/workflows/Rust/badge.svg)

Provides utility to dump novel from the japanese web novel publishing site.

## Usage

```
Utility to download text of the syosetu novels

USAGE: [OPTIONS] <novel>

OPTIONS:
    -h,  --help         Prints this help information
         --from <from>  Specify from which chapter to start dumping. Default: 1.
         --to <to>      Specify until which chapter to dump.

ARGS:
    <novel>  Id of the novel to dump (e.g. n9185fm)
```

## Convert to EPUB

I recommend to use [pandoc](https://github.com/jgm/pandoc):

```
pandoc --embed-resources --standalone --metadata title="TITLE" -o TITLE.epub TITLE.md
```
