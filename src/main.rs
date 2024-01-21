#![no_main]
#![allow(clippy::result_large_err)]

use std::{io, fs, path};

mod cli;
mod data;

#[inline(always)]
fn get(path: &str) -> Result<ureq::Response, ureq::Error> {
    //over18=yes;
    ureq::get(path).timeout(core::time::Duration::from_secs(5))
                   .set("Cookie", "over18=yes")
                   .call()
}

#[no_mangle]
fn rust_main(args: c_main::Args) -> bool {
    let args = match cli::Cli::new(args.into_iter().skip(1)) {
        Ok(args) => args,
        Err(code) => return code,
    };

    let api_endpoint = match args.r18 {
        true => "novel18api",
        false => "novelapi",
    };

    let resp = match get(&format!("https://api.syosetu.com/{api_endpoint}/api/?out=json&ncode={}", args.novel.0)) {
        Ok(resp) => if resp.status() != 200 {
            eprintln!("Request to api.syosetu.com failed with code: {}", resp.status());
            return false;
        } else {
            resp
        },
        Err(ureq::Error::Status(code, _)) => {
            eprintln!("Request to api.syosetu.com failed with code: {}", code);
            return false;
        },
        Err(ureq::Error::Transport(_)) => {
            eprintln!("api.syosetu.com is unreachable");
            return false
        },
    };

    let (_, info) = match resp.into_json::<data::NovelInfo>() {
        Ok(mut info) => {
            info.1.ncode.make_ascii_lowercase();
            info
        }
        Err(error) => {
            eprintln!("Failed to get novel '{}' info. Error: {}", args.novel.0, error);
            return false
        }
    };

    println!("Novel: ");
    println!("Title={}", info.title);
    println!("Code={}", info.ncode);
    println!("Author={}", info.writer);
    println!("Chapter Number={}", info.chapter_count);
    println!("Last Updated={}", info.updated_at);

    if args.from.get() > info.chapter_count {
        eprintln!("From is '{}' but novel has only {} chapters", args.from, info.chapter_count);
        return false;
    }

    let to = if let Some(to) = args.to {
        let to = to.get();
        if to > info.chapter_count {
        eprintln!("To is '{}' but novel has only {} chapters", args.from, info.chapter_count);
        } else if args.from.get() > to {
            eprintln!("From '{}' is above To '{}, which is retarded", args.from, to);
            return false;
        }

        to
    } else {
        info.chapter_count
    };

    let mut file = match fs::File::create(construct_file_path(".", &info.title)) {
        Ok(file) => io::BufWriter::new(file),
        Err(error) => {
            eprintln!("Failed to create file to store content. Error: {}", error);
            return false;
        },
    };

    if let Err(error) = io::Write::write_fmt(&mut file, format_args!("# {}\n\nOriginal: https://ncode.syosetu.com/{}\n\n", info.title, info.ncode)) {
        eprintln!("Unable to write file: {}", error);
        return false;
    }

    for idx in args.from.get()..=to {
        print!("Downloading chapter {} ({}/{})...", idx, info.ncode, idx);
        let text = loop {
            let resp = match get(&format!("https://ncode.syosetu.com/{}/{}", info.ncode, idx)) {
                Ok(resp) => if resp.status() != 200 {
                    println!("ERR");
                    eprintln!("Request to ncode.syosetu.com failed with code: {}", resp.status());
                    continue
                } else {
                    resp
                },
                Err(ureq::Error::Status(code, _)) => {
                    println!("ERR");
                    eprintln!("Request to ncode.syosetu.com failed with code: {}", code);
                    continue
                },
                Err(ureq::Error::Transport(_)) => {
                    eprintln!("ncode.syosetu.com is unreachable");
                    continue
                },
            };

            match resp.into_string() {
                Ok(text) => break text,
                Err(error) => {
                    println!("ERR");
                    eprintln!("Unable to get content of chapter: {}", error);
                }
            }
        };

        if let Err(error) = dump(&mut file, &text) {
            println!("ERR");
            eprintln!("Failed to store novel. Error: {}", error);
            return false;
        }

        println!("OK");
    }

    let _ = io::Write::flush(&mut file);

    true
}

fn construct_file_path(dir: &str, name: &str) -> path::PathBuf {
    let mut path = path::PathBuf::from(dir);
    path.push(name);
    path.set_extension("md");

    path
}

fn dump<W: io::Write>(dest: &mut W, html: &str) -> io::Result<()> {
    use kuchiki::traits::TendrilSink;

    const WHITE_SPACE: &[char] = &[' ', '\t', '\n', '　'];
    const NOVEL_BODY: &str = "#novel_honbun";
    const NOVEL_TITLE: &str = ".novel_subtitle";

    let document = kuchiki::parse_html().from_utf8().one(html.as_bytes());

    let novel_title = match document.select_first(NOVEL_TITLE) {
        Ok(node) => node,
        Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "Unable to find #novel_honbun block")),
    };
    let novel_title = novel_title.as_node();

    let novel_text = match document.select_first(NOVEL_BODY) {
        Ok(node) => node,
        Err(_) => return Err(io::Error::new(io::ErrorKind::Other, "Unable to find .novel_view block")),
    };
    let novel_text = novel_text.as_node();

    dest.write_fmt(format_args!("## {}\n", novel_title.text_contents()))?;

    for child in novel_text.children() {
        if let Some(element) = child.into_element_ref() {
            let text = element.text_contents();
            let text = text.trim_matches(WHITE_SPACE);
            dest.write_all(b"\n")?;

            if !text.is_empty() {
                dest.write_all(text.as_bytes())?;
            } else if let Ok(img) = element.as_node().select_first("img") {
                let img = img.attributes.borrow();
                if let Some(src) = img.get("src") {
                    let alt = img.get("alt").unwrap_or("");
                    dest.write_fmt(format_args!("![{alt}]({src})"))?;
                }
            }
        }
    }

    dest.write_all(b"\n\n")?;

    Ok(())
}
