#!/usr/bin/env python3

# This requires both of the following to be installed:
#
# - Pandoc: https://pandoc.org
# - Kepubify: https://pgaskin.net/kepubify/
#
# It downloads books from https://syosetu.com and converts them into
# .kepub format for Kobo e-readers.  Kepub is also compatible with
# standard epub files, so they should work on any e-reader that
# supports epub files as well.

EPUB_CSS = """@charset "utf-8";
body {
    writing-mode: vertical-rl;
    -webkit-writing-mode: vertical-rl;
    -moz-writing-mode: vertical-rl;
    -o-writing-mode: vertical-rl;
    -ms-writing-mode: vertical-rl;
    -epub-writing-mode: vertical-rl;

    text-orientation: upright;
    -webkit-text-orientation: upright;
    -moz-text-orientation: upright;
    -o-text-orientation: upright;
    -ms-text-orientation: upright;
    -epub-text-orientation: upright;

    font-size: medium;
    font-family: serif;
    text-align: justify;
    margin: 4% 4%;
    line-height: 1.75
}

div, span, p, img, nav, section, h1, h2, h3, h4, h5, h6 {
    padding: 0;
    border: 0;
    outline: 0;
    vertical-align: baseline;
}
div, span, p, img, nav, section { margin: 0; font-weight: normal; }

h1, h2, h3, h4, h5, h6 { font-weight: bold; }
h1 { font-size: 1.5em; margin-left: 1.5em; }
h2 { font-size: 1.3em; margin-left: 1.3em; }
h3 { font-size: 1.2em; margin-left: 1.2em; }
h4 { font-size: 1.1em; margin-left: 1.1em; }
h5 { font-size: 1.0em; margin-left: 1.0em; }
h6 { font-size: 1.0em; margin-left: 1.0em; }

p {}
p.blank {
    width: 1.0em;
    height: 1.0em;
}
strong { font-weight: bold; }
em { font-style: italic;}
code{ white-space: pre-wrap; font-family: monospace; }
q { quotes: "“" "”" "‘" "’"; }

/* For title/cover page. */
section.titlepage {
    margin: 1.5em;
}
h1.title {
}
p.author {
}
p.date {
}

nav#toc ol,
nav#landmarks ol { padding: 0; margin-left: 1em; }
nav#toc ol li,
nav#landmarks ol li { list-style-type: none; margin: 0; padding: 0; }
a.footnote-ref { vertical-align: super; }
span.smallcaps{ font-variant: small-caps; }
span.underline{ text-decoration: underline; }
div.column{ display: inline-block; vertical-align: top; width: 50%; }

/* Misc classes for special styling. */
.horiz {
    writing-mode: horizontal-tb;
    -webkit-writing-mode: horizontal-lr;
    -moz-writing-mode: horizontal-lr;
    -o-writing-mode: horizontal-lr;
    -ms-writing-mode: horizontal-lr;
    -epub-writing-mode: horizontal-lr;
}

.inset {
    margin-top: 3.0em;
    margin-bottom: 3.0em;
}

.box {
    margin: 1.0em;
    padding 1.0em;
    border: 1px solid #000;
}
"""

import argparse
import sys
import os
import shutil
import time
import tempfile
import subprocess
import urllib.request
import re


def get_page(url, timeout=10.0):
    # IP will be banned for a short time if pages are loaded too fast.
    # The original script had a wait time of 0.1 seconds, which worked
    # fine.  0.5 is extra conservative, just to be safe.
    time.sleep(0.5)

    # We fake being a browser, because syosetu.com returns 403 otherwise.
    headers = {'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.95 Safari/537.36'}
    req = urllib.request.Request(url, headers=headers)

    return urllib.request.urlopen(req, timeout=timeout).read().decode('utf8')

def maybe_group(match, group_index):
    if match != None:
        return match.group(group_index)
    else:
        return ""

# Common substitutions.
def common_subs(text):
    subs = [
        ["0", "０"],
        ["1", "１"],
        ["2", "２"],
        ["3", "３"],
        ["4", "４"],
        ["5", "５"],
        ["6", "６"],
        ["7", "７"],
        ["8", "８"],
        ["9", "９"],
        ["&quot;", "\""],
    ]
    for sub in subs:
        text = text.replace(sub[0], sub[1])
    return text

def generate_chapters_text(chapters, title_prefix="#"):
    text = ""
    for chapter_page in chapters:
        chapter_title = common_subs(maybe_group(re.search("(?ms)<p class=\"novel_subtitle\">(.*?)</p>", chapter_page), 1).strip())
        text += "{} {}\n\n".format(title_prefix, chapter_title)
        chapter_text = maybe_group(re.search("(?ms)<div id=\"novel_honbun\" class=\"novel_view\">(.*?)</div>", chapter_page), 1).strip()
        for paragraph in re.finditer("(?ms)<p[^>]*>(.*?)</p>", chapter_text):
            paragraph = paragraph.group(1).strip()
            if paragraph == "<br>" or paragraph == "<br/>" or paragraph == "<br />":
                # We do this because authors on syosetu.com really love
                # to overuse <br/> tags.  Combined with the styling of
                # p.blank, this keeps the spacing not completely crazy.
                text += "\n\n<p class=\"blank\"></p>"
            elif paragraph != "":
                text += "\n\n{}".format(common_subs(paragraph))
        text += "\n\n\n"

    return text


if __name__ == "__main__":
    arg_parser = argparse.ArgumentParser(description=
        """
        Downloads books from syosetu.com and converts them to .epub format.
        """
    )
    arg_parser.add_argument("-l", "--local", help="Just convert a local markdown file instead of downloading anything.", action="store_true")
    arg_parser.add_argument("-k", "--kepub", help="Convert to Kobo kepub instead of plain epub (requires Kepubify to be installed).", action="store_true")
    arg_parser.add_argument("-v", "--volume", help="For books with multiple volumes, this specifies the volume to download.")
    arg_parser.add_argument("-t", "--title", help="Specify an alternate title to use (sometimes the titles have extra non-title info in them on the site).")
    arg_parser.add_argument("book", help="The full url of book's main page on syosetu.com, or path to markdown file if using -l flag.")
    args = arg_parser.parse_args()
    
    # The book text and output filename (sans extension).  Built below.
    text = ""
    book_filename = ""

    if args.local:
        filepath = args.book
        with open(filepath, mode='r') as markdown:
            text = markdown.read()
        book_filename = re.sub("\.[^\.]*", "", args.book)
    else:
        main_url = args.book
        if main_url.endswith("/"):
            main_url = main_url[:-1]

        # Download main page.
        print("Downloading main page...")
        main_page = get_page(main_url)

        # Extract book info.
        if args.title != None:
            title = args.title
        else:
            title = common_subs(maybe_group(re.search("(?ms)<p class=\"novel_title\">(.*?)</p>", main_page), 1).strip())
        if args.volume != None:
            title += "（v{}）".format(args.volume)
        author = maybe_group(re.search("(?ms)<div class=\"novel_writername\">.*?作者：(.*?)</div>", main_page), 1).strip()
        author = re.sub("<a[^>]*>", "", author).strip()
        author = common_subs(re.sub("</a>", "", author).strip())
        # summary = maybe_group(re.search("(?ms)<div id=\"novel_ex\">(.*?)</div>", main_page), 1).strip()

        # Get the list of chapters, possibly organized by volume.
        volume_list = re.findall("(?ms)<div class=\"chapter_title\">(.*?)</div>", main_page);
        if len(volume_list) > 1:
            volumes = re.compile("(?ms)<div class=\"chapter_title\">.*?</div>").split(main_page)[1:]
            for i in range(len(volume_list)):
                volume_list[i] = [
                    volume_list[i].strip(),
                    re.findall("(?ms)<dd class=\"subtitle\">(.*?)</dd>", volumes[i]),
                ]
        else:
            volume_list = [["", re.findall("(?ms)<dd class=\"subtitle\">(.*?)</dd>", main_page)]]

        # Limit to just a single volume.
        if args.volume != None:
            n = int(args.volume) - 1
            volume_list = [volume_list[n]]

        print("Title: ", title)
        print("Author: ", author)

        # Download chapter pages.
        volumes = []
        for i in range(len(volume_list)):
            volumes += [[volume_list[i][0], []]]
            for j in range(len(volume_list[i][1])):
                print("Downloading volume \"{}\" ({}/{}) chapter {}/{}".format(
                    volume_list[i][0],
                    i + 1,
                    len(volume_list),
                    j + 1,
                    len(volume_list[i][1]),
                ))
                sub_chapter_url_number = re.findall("(?ms)href=\"/[^/]*/([0-9]+)", volume_list[i][1][j])[0]
                sub_chapter_url = "{}/{}".format(main_url, sub_chapter_url_number)
                volumes[i][1] += [get_page(sub_chapter_url)]

        text += "---\n"
        text += "title:\n"
        text += "- type: main\n"
        text += "  text: {}\n".format(title)
        if len(volumes) == 1 and volumes[0][0] != "":
            text += "- type: subtitle\n"
            text += "  text: {}\n".format(volumes[0][0])
        text += "author: {}\n".format(author)
        text += "language: ja\n"
        text += "---\n\n"

        if len(volumes) == 1:
            text += generate_chapters_text(volumes[0][1], "#")
        else:
            for volume in volumes:
                text += "# {}\n\n".format(volume[0])
                text += generate_chapters_text(volume[1], "##")

    # Create the epub/kepub file via pandoc and kepubify.
    with tempfile.TemporaryDirectory() as tmpdir_path:
        css_filepath = os.path.join(tmpdir_path, "book_style.css")
        book_text_filepath = os.path.join(tmpdir_path, "book.md")
        book_epub_filepath = os.path.join(tmpdir_path, "book.epub")
        book_kepub_filepath = os.path.join(tmpdir_path, "book.kepub.epub")

        with open(css_filepath, mode='w', encoding="utf8") as css:
            css.write(EPUB_CSS)
        with open(book_text_filepath, mode='w', encoding="utf8") as book_text:
            book_text.write(text)

        subprocess.run([
            "pandoc",
            book_text_filepath,
            "--css", css_filepath,
            "-o", book_epub_filepath,
        ])

        if args.kepub:
            subprocess.run([
                "kepubify",
                book_epub_filepath,
                "-o",
                book_kepub_filepath,
            ])

        if book_filename == "":
            book_filename = title.replace("/", "").replace("\\", "").strip()

        # shutil.copyfile(book_text_filepath, "./{}.md".format(book_filename))
        if args.kepub:
            shutil.copyfile(book_kepub_filepath, "./{}.kepub.epub".format(book_filename))
        else:
            shutil.copyfile(book_epub_filepath, "./{}.epub".format(book_filename))
