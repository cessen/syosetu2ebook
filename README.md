# Syosetu 2 eBook

https://syosetu.com is a Japanese website where aspiring and hobby
authors publish their books for free.  Unfortunately, they do not offer e-book
downloads, only online reading and PDF downloads, which makes reading the books
on an e-reader somewhat obnoxious.

To solve that problem, this program will automatically download and convert novels
from syosetu.com into epub format for you.


## Building

Ensure that you have the standard [Rust](https://www.rust-lang.org) toolchain
installed.  Then from the repository root simply run:

```
cargo build --release
```


## Usage

```bash
./syosetu2ebook <main_url>
```

Where `<main_url>` is the url of the main page of the book.

For example, to download 転生したらスライムだった件 (http://ncode.syosetu.com/n6316bn),
simply run:

```bash
./syosetu2ebook http://ncode.syosetu.com/n6316bn
```

In this case, there are multiple volumes, and an epub file will be generated
for each volume.  If you want just a specific volume, you can specify that with
`-v`.  For example, if you want to download just the third volume:

```bash
./syosetu2ebook -v 3 http://ncode.syosetu.com/n6316bn
```

For books that are just one long stream of a huge number of chapters, you can
limit which chapters are downloaded as well.  For example, to download just
volume 2, chapters 5 through 10:

```bash
./syosetu2ebook -v 2 -c 5-10 http://ncode.syosetu.com/n6316bn
```

You can also optionally have furigana automatically generated (not 100%
accurate, but is pretty good) by passing the `-f` flag:

```bash
./syosetu2ebook -f http://ncode.syosetu.com/n6316bn
```

There are additional features as well.  Please see the command line help
(`./syosetu2ebook --help`) for more details.


## License

Syosetu 2 eBook is licensed under the MIT license (LICENSE or http://opensource.org/licenses/MIT).


## Contributing

Contributions are absolutely welcome!  If you want to make larger changes,
please first open an issue to discuss it to avoid doing a lot of work that may
get rejected.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Syosetu 2 eBook by you will be licensed as above, without any
additional terms or conditions.
