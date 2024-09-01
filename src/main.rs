//! This requires both of the following to be installed:
//!
//! - Pandoc: https://pandoc.org
//! - Kepubify: https://pgaskin.net/kepubify/
//!
//! It downloads books from https://syosetu.com and converts them into
//! .kepub format for Kobo e-readers.  Kepub is also compatible with
//! standard epub files, so they should work on any e-reader that
//! supports epub files as well.

use std::{
    fs::File,
    io::{Read, Write},
    process::Command,
    time::Duration,
};

use furigana_gen::FuriganaGenerator;

const EPUB_CSS: &str = r#"@charset "utf-8";
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
"#;

fn get_page(url: &str) -> Result<String, ureq::Error> {
    const TIMEOUT_SECS: u64 = 60;

    // IP will be banned for a short time if pages are loaded too fast.
    // The original script had a wait time of 0.1 seconds, which worked
    // fine.  1.0 is extra conservative, just to be safe.
    std::thread::sleep(Duration::from_secs_f32(0.5));

    let agent: ureq::Agent = ureq::AgentBuilder::new()
      .timeout(Duration::from_secs(TIMEOUT_SECS))
      // We fake being a browser, because syosetu.com returns 403 otherwise.
      .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/39.0.2171.95 Safari/537.36")
      .build();

    Ok(agent.get(url).call()?.into_string().unwrap())
}

fn maybe_group<'a>(hit: Option<regex::Captures<'a>>, group_index: usize) -> &'a str {
    if let Some(hit) = hit {
        hit.get(group_index).map(|m| m.as_str()).unwrap_or("")
    } else {
        ""
    }
}

/// Common substitutions.
fn common_subs(text: &str) -> String {
    const SUBS: &[[&str; 2]] = &[
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
    ];

    let mut new_text: String = text.into();
    for sub in SUBS {
        new_text = new_text.replace(sub[0], sub[1]);
    }

    new_text
}

fn generate_chapter_md(
    chapter_html: &str,
    title_prefix: &str,
    furigana_generator: Option<&FuriganaGenerator>,
) -> String {
    let mut text = String::new();

    let re_title = regex::Regex::new(r#"(?ms)<p class=\"novel_subtitle\">(.*?)</p>"#).unwrap();
    let chapter_title = maybe_group(re_title.captures(chapter_html), 1).trim();

    text.push_str(&format!("{} {}\n\n", title_prefix, chapter_title));

    let re_text =
        regex::Regex::new(r#"(?ms)<div id=\"novel_honbun\" class=\"novel_view\">(.*?)</div>"#)
            .unwrap();
    let chapter_text = maybe_group(re_text.captures(chapter_html), 1).trim();

    let re_paragraph = regex::Regex::new(r#"(?ms)<p[^>]*>(.*?)</p>"#).unwrap();
    for paragraph in re_paragraph
        .captures_iter(chapter_text)
        .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or("").trim())
    {
        if paragraph == "<br>" || paragraph == "<br/>" || paragraph == "<br />" {
            // We do this because authors on syosetu.com really love
            // to overuse <br/> tags.  Combined with the styling of
            // p.blank, this keeps the spacing not completely crazy.
            text.push_str("\n\n<p class=\"blank\"></p>");
        } else if paragraph != "" {
            text.push_str("\n\n");
            text.push_str(paragraph);
        }
    }
    text.push_str("\n\n\n");

    text = common_subs(&text);

    // Optionally add furigana.
    if let Some(furigen) = furigana_generator {
        text = furigen.add_html_furigana(&text);
    }

    text
}

#[derive(Clone, Debug)]
struct Args {
    local: bool,
    kepub: bool,
    furigana: bool,
    volume: Option<usize>,
    title: Option<String>,
    book: String,
}

impl Args {
    fn parse() -> Args {
        use bpaf::{construct, positional, short, Parser};

        let local = short('l')
            .long("local")
            .help("Just convert a local markdown file instead of downloading anything.")
            .switch();
        let kepub = short('k')
            .long("kepub")
            .help(
                "Convert to Kobo kepub instead of plain epub (requires Kepubify to be installed).",
            )
            .switch();
        let furigana = short('f')
            .long("furigana")
            .help("Auto-generate furigana on kanji in the text.")
            .switch();
        let volume = short('v')
            .long("volume")
            .help("For books with multiple volumes, this specifies the volume to download.")
            .argument::<usize>("VOLUME")
            .optional();
        let title = short('t')
        .long("title")
        .help("Specify an alternate title to use (sometimes the titles have extra non-title info in them on the site).")
        .argument::<String>("TITLE").optional();
        let book = positional::<String>("BOOK")
        .help("The full url of book's main page on syosetu.com, or path to markdown file if using -l flag.");

        construct!(Args {
            local,
            kepub,
            furigana,
            volume,
            title,
            book
        })
        .to_options()
        .run()
    }
}

fn main() {
    let args = Args::parse();

    let furigana_generator = if args.furigana {
        Some(FuriganaGenerator::new())
    } else {
        None
    };

    // The book text and output filename (sans extension).  Built below.
    let mut text = String::new();
    let mut book_filename: String;

    if args.local {
        let mut f = File::open(&args.book).unwrap();
        f.read_to_string(&mut text).unwrap();
        book_filename = regex::Regex::new(r#"\.[^\.]*$"#)
            .unwrap()
            .replace_all(&args.book, "")
            .into();
        book_filename = regex::Regex::new(r#"^.*/"#)
            .unwrap()
            .replace_all(&book_filename, "")
            .into();
    } else {
        let main_url = args.book.trim_end_matches("/");

        // Download main page.
        //
        // TODO: handle paginated main pages.
        println!("Downloading main page...");
        let main_page = get_page(&main_url).unwrap();

        // Extract book info.
        let title = {
            let mut title = if let Some(title) = args.title {
                title
            } else {
                let re = regex::Regex::new(r#"(?ms)<p class=\"novel_title\">(.*?)</p>"#).unwrap();
                common_subs(maybe_group(re.captures(&main_page), 1).trim())
            };

            if let Some(vol) = args.volume {
                title.push_str(&format!(" (vol {})", vol));
            }

            title
        };
        let author = {
            let re1 =
                regex::Regex::new(r#"(?ms)<div class=\"novel_writername\">.*?作者：(.*?)</div>"#)
                    .unwrap();
            let re2 = regex::Regex::new(r#"<a[^>]*>"#).unwrap();

            let mut author: String = maybe_group(re1.captures(&main_page), 1).trim().into();
            author = re2.replace_all(&author, "").trim().into();
            author = author.replace("</a>", "").trim().into();
            author
        };
        // let summary = {
        //     let re = regex::Regex::new(r#"(?ms)<div id=\"novel_ex\">(.*?)</div>"#).unwrap();
        //     maybe_group(re.captures(&main_page), 1).trim()
        // };

        println!("Title: {}", title);
        println!("Author: {}", author);
        // println!("Summary: {}", summary);

        book_filename = title.replace("/", "").replace("\\", "").trim().into();

        // Get the list of chapters, possibly organized by volume.
        //
        // A vector of (volume_title, chapter_links), where the chapter links are
        // in `<a href="url">title</a>` format.
        let volume_list: Vec<(&str, Vec<&str>)> = {
            let re_volumes =
                regex::Regex::new(r#"(?ms)<div class=\"chapter_title\">(.*?)</div>"#).unwrap();

            fn get_chapter_links<'a>(html: &'a str) -> Vec<&'a str> {
                let re_chapters =
                    regex::Regex::new(r#"(?ms)<dd class=\"subtitle\">(.*?)</dd>"#).unwrap();

                re_chapters
                    .captures_iter(html)
                    .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or("").trim())
                    .collect()
            }

            let volume_titles: Vec<&str> = re_volumes
                .captures_iter(&main_page)
                .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or("").trim())
                .collect();

            let volume_list: Vec<_> = if volume_titles.len() > 1 {
                let volume_htmls: Vec<&str> = re_volumes.split(&main_page).skip(1).collect();

                volume_titles
                    .iter()
                    .zip(volume_htmls.iter())
                    .map(|(&title, html)| (title, get_chapter_links(html)))
                    .collect()
            } else {
                vec![("", get_chapter_links(&main_page))]
            };

            if let Some(vol) = args.volume {
                // Limit to just the single specified volume.
                let n = vol - 1;
                [volume_list[n].clone()].into()
            } else {
                volume_list
            }
        };

        // Download chapter pages.
        let volumes: Vec<(&str, Vec<String>)> = {
            let re_chapter_number = regex::Regex::new(r#"(?ms)href=\"/[^/]*/([0-9]+)"#).unwrap();

            let mut volumes: Vec<(&str, Vec<String>)> = Vec::new();

            for i in 0..volume_list.len() {
                let mut chapters: Vec<String> = Vec::new();
                for (j, chapter_link) in volume_list[i].1.iter().enumerate() {
                    println!(
                        "Downloading volume \"{}\" ({}/{}) chapter {}/{}",
                        volume_list[i].0,
                        i + 1,
                        volume_list.len(),
                        j + 1,
                        volume_list[i].1.len(),
                    );

                    let sub_chapter_url_number =
                        maybe_group(re_chapter_number.captures(chapter_link), 1);
                    let sub_chapter_url = format!("{}/{}", main_url, sub_chapter_url_number);
                    let chapter_html = get_page(&sub_chapter_url).unwrap();
                    let title_prefix = if volume_list.len() > 1 { "##" } else { "#" };
                    chapters.push(generate_chapter_md(
                        &chapter_html,
                        title_prefix,
                        furigana_generator.as_ref(),
                    ));
                }

                volumes.push((volume_list[i].0, chapters));
            }

            volumes
        };

        text.push_str("---\n");
        text.push_str("title:\n");
        text.push_str("- type: main\n");
        text.push_str(&format!("  text: {}\n", title));
        if volumes.len() == 1 && volumes[0].0 != "" {
            text.push_str("- type: subtitle\n");
            text.push_str(&format!("  text: {}\n", volumes[0].0));
        }
        text.push_str(&format!("author: {}\n", author));
        text.push_str("language: ja\n");
        text.push_str("---\n\n");

        if volumes.len() == 1 {
            for chapter_md in &volumes[0].1 {
                text.push_str(chapter_md);
            }
        } else {
            for volume in &volumes {
                text.push_str(&format!("# {}\n\n", volume.0));
                for chapter_md in &volume.1 {
                    text.push_str(chapter_md);
                }
            }
        }
    }

    if args.furigana {
        book_filename.push_str("_furigana");
    }

    // Create the epub/kepub file via pandoc and kepubify.
    let tmpdir = tempfile::tempdir().unwrap();
    let css_filepath = tmpdir.path().join("book_style.css");
    let book_md_filepath = tmpdir.path().join("book.md");
    let book_epub_filepath = tmpdir.path().join("book.epub");
    let book_kepub_filepath = tmpdir.path().join("book.kepub.epub");

    {
        let mut f = File::create(&css_filepath).unwrap();
        f.write_all(EPUB_CSS.as_bytes()).unwrap();
    }
    {
        let mut f = File::create(&book_md_filepath).unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    if !args.local {
        std::fs::copy(&book_md_filepath, format!("./{}.md", &book_filename)).unwrap();
    }

    {
        let output = Command::new("pandoc")
            .arg(&book_md_filepath)
            .arg("--css")
            .arg(&css_filepath)
            .arg("-o")
            .arg(&book_epub_filepath)
            .output()
            .expect("Failed to execute pandoc: are you sure it's installed and in your path?");

        std::io::stdout().write_all(&output.stdout).unwrap();
        if !output.status.success() {
            std::io::stderr().write_all(&output.stderr).unwrap();
            panic!("pandoc did not succeed.");
        }
    }
    if args.kepub {
        let output = Command::new("kepubify")
            .arg(&book_epub_filepath)
            .arg("-o")
            .arg(&book_kepub_filepath)
            .output()
            .expect("Failed to execute kepubify: are you sure it's installed and in your path?");

        std::io::stdout().write_all(&output.stdout).unwrap();
        if !output.status.success() {
            std::io::stderr().write_all(&output.stderr).unwrap();
            panic!("kepubify did not succeed.");
        }
    }

    if args.kepub {
        std::fs::copy(
            book_kepub_filepath,
            format!("./{}.kepub.epub", book_filename),
        )
        .unwrap();
    } else {
        std::fs::copy(book_epub_filepath, format!("./{}.epub", book_filename)).unwrap();
    }
}
