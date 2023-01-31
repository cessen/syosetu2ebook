# Syosetu 2 eBook

https://syosetu.com is a Japanese website where aspiring and hobby
authors publish their books for free.  Unfortunately, they do not offer e-book
downloads, only online reading and PDF downloads, which makes reading the books
on an e-reader somewhat obnoxious.

To solve that problem, this script will automatically download and convert novels
from syosetu.com into epub or kepub for you.


## Requirements

- Python 3.x
- [Pandoc](https://pandoc.org): to convert .md -> .epub
- [Kepubify](https://pgaskin.net/kepubify/): to convert .epub -> .kepub.epub
  (only needed for Kobo kepub support)


## Usage

```bash
./yomou2ebook.py <main_url>
```

Where `<main_url>` is the url of the main page of the book, for example:
http://ncode.syosetu.com/n6316bn

For example, to download 転生したらスライムだった件 (http://ncode.syosetu.com/n6316bn), simply run:
```bash
./yomou2ebook.py http://ncode.syosetu.com/n6316bn
```

Both a `転生したらスライムだった件.md` markdown file and a `転生したらスライムだった件.epub` file will be generated.  The latter file should work on all e-readers that support .epub format.

You can optionally pass the `-k` flag to generate a Kobo kepub file instead:

```bash
./yomou2ebook.py -k <main_url>
```


## License

Syosetu 2 eBook is licensed under the MIT license (LICENSE or http://opensource.org/licenses/MIT).


## Contributing

Contributions are absolutely welcome!  If you want to make larger changes, please first open an issue to discuss it to avoid doing a lot of work that may get rejected.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Syosetu 2 eBook by you will be licensed as above, without any additional terms or conditions.
