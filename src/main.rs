#![allow(clippy::style, clippy::result_large_err)]

use std::{io, fs, path};
use std::process::ExitCode;
use core::num::NonZeroUsize;
use core::time;

mod cli;
mod data;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn args_from_stdin() -> Result<cli::Cli, ExitCode> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    macro_rules! prompt {
        ($($arg:tt)*) => {
            let _ = io::Write::write_fmt(&mut stdout, format_args!($($arg)*));
            let _ = io::Write::flush(&mut stdout);
        }
    }

    macro_rules! read_line {
        () => {
            buffer.clear();
            if let Err(error) = stdin.read_line(&mut buffer) {
                eprintln!("!>>>Unexpected I/O error: {error}");
                return Err(ExitCode::FAILURE);
            }
        };
    }

    let novel: cli::Id;
    loop {
        prompt!(">Please input novel id (e.g. n9185fm): ");
        read_line!();

        let line = buffer.trim();
        if line.is_empty() {
            continue;
        }

        match line.parse() {
            Ok(new_id) => {
                novel = new_id;
                break;
            },
            Err(error) => {
                eprintln!("!>>>{error}");
                continue;
            }
        }
    }

    prompt!(">Is novel 18+?(y/N):");
    read_line!();

    let line = buffer.trim();
    let r18 = if line.is_empty() {
        false
    } else {
        line.eq_ignore_ascii_case("y") || line.eq_ignore_ascii_case("yes")
    };

    let from;
    prompt!(">Please specify which chapters to download:\n");
    loop {
        prompt!("Start FROM chapter(defaults to 1)?:");
        read_line!();

        let line = buffer.trim();
        if line.is_empty() {
            from = cli::default_from_value();
            break;
        }

        match usize::from_str_radix(&line, 10) {
            Ok(chapter) => match NonZeroUsize::new(chapter) {
                Some(chapter) => {
                    from = chapter;
                    break;
                },
                None => {
                    eprintln!("!>>>Chapter cannot be zero");
                    continue
                }
            },
            Err(error) => {
                eprintln!("!>>>'{line}': {error}");
                continue;
            }
        }
    }

    let to;
    loop {
        prompt!("TO chapter(leave empty for all)?:");
        read_line!();

        let line = buffer.trim();
        if line.is_empty() {
            to = None;
            break;
        }

        match usize::from_str_radix(line, 10) {
            Ok(chapter) => if chapter > from.get() {
                to = Some(unsafe {
                    NonZeroUsize::new_unchecked(chapter)
                });
                break;
            } else {
                eprintln!("!>>>Number has to be greater than from='{from}'");
                continue
            },
            Err(error) => {
                eprintln!("!>>>{error}");
                continue;
            }
        }
    }

    Ok(cli::Cli {
        from,
        r18,
        to,
        novel,
        title: None,
    })
}

fn run(args: cli::Cli) -> ExitCode {
    let config = ureq::Agent::config_builder().user_agent(USER_AGENT)
                                              .proxy(ureq::Proxy::try_from_env())
                                              .max_redirects(0)
                                              .timeout_per_call(Some(time::Duration::from_secs(5)))
                                              .timeout_connect(Some(time::Duration::from_secs(1)))
                                              .build();
    let http_client = ureq::Agent::new_with_config(config);

    let api_endpoint = match args.r18 {
        true => "novel18api",
        false => "novelapi",
    };

    let resp = http_client.get(&format!("https://api.syosetu.com/{api_endpoint}/api/?out=json&ncode={}", args.novel.0)).header("Cookie", "over18=yes").call();

    let mut resp = match resp {
        Ok(resp) => if resp.status().as_u16() != 200 {
            eprintln!("Request to api.syosetu.com failed with code: {}", resp.status());
            return ExitCode::FAILURE;
        } else {
            resp
        },
        Err(ureq::Error::StatusCode(code)) => {
            eprintln!("Request to api.syosetu.com failed with code: {}", code);
            return ExitCode::FAILURE;
        },
        Err(error) => {
            eprintln!("api.syosetu.com is unreachable: {error}");
            return ExitCode::FAILURE;
        },
    };

    let response = match resp.body_mut().read_to_string() {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Failed to get novel '{}' info. Error: {}", args.novel.0, error);
            return ExitCode::FAILURE;
        }
    };

    let info = match serde_json::from_str::<Vec<data::NovelInfo>>(&response) {
        Ok(mut info) => match info.pop() {
            Some(data::NovelInfo::Info(mut info)) => {
                info.ncode.make_ascii_lowercase();
                info
            },
            _ => {
                eprintln!("Novel '{}' is not found", args.novel.0);
                return ExitCode::FAILURE;
            }
        },
        Err(error) => {
            eprintln!("Failed to get novel '{}' info. Invalid JSON: {}", args.novel.0, error);
            eprintln!("JSON:\n{response}");
            return ExitCode::FAILURE;
        },
    };

    println!("## Novel: ");
    println!("Title={}", info.title);
    println!("Code={}", info.ncode);
    println!("Author={}", info.writer);
    println!("Chapter Number={}", info.chapter_count);
    println!("Last Updated={}", info.updated_at);

    if args.from.get() > info.chapter_count {
        eprintln!("From is '{}' but novel has only {} chapters", args.from, info.chapter_count);
        return ExitCode::FAILURE;
    }

    let title = match args.title {
        Some(title) => title,
        None => info.title,
    };
    let to = if let Some(to) = args.to {
        let to = to.get();
        if to > info.chapter_count {
        eprintln!("To is '{}' but novel has only {} chapters", args.from, info.chapter_count);
        } else if args.from.get() > to {
            eprintln!("From '{}' is above To '{}, which is retarded", args.from, to);
            return ExitCode::FAILURE;
        }

        to
    } else {
        info.chapter_count
    };

    let mut file = match fs::File::create(construct_file_path(".", &title)) {
        Ok(file) => io::BufWriter::new(file),
        Err(error) => {
            eprintln!("Failed to create file to store content. Error: {}", error);
            return ExitCode::FAILURE;
        },
    };

    let host_prefix = match args.r18 {
        true => "novel18",
        false => "ncode",
    };
    if let Err(error) = io::Write::write_fmt(&mut file, format_args!("# {}\n\nOriginal: https://{host_prefix}.syosetu.com/{}\n\n", title, info.ncode)) {
        eprintln!("Unable to write file: {}", error);
        return ExitCode::FAILURE;
    }
    for idx in args.from.get()..=to {
        print!("Downloading chapter {} ({}/{})...", idx, info.ncode, idx);
        let text = loop {

            let resp = http_client.get(&format!("https://{host_prefix}.syosetu.com/{}/{}", info.ncode, idx)).header("Cookie", "over18=yes").call();
            let mut resp = match resp {
                Ok(resp) => if resp.status().as_u16() != 200 {
                    println!("ERR");
                    eprintln!("Request to ncode.syosetu.com failed with code: {}", resp.status());
                    continue
                } else {
                    resp
                },
                Err(ureq::Error::StatusCode(code)) => {
                    println!("ERR");
                    eprintln!("Request to {host_prefix}.syosetu.com failed with code: {code}");
                    continue
                },
                Err(error) => {
                    eprintln!("ncode.syosetu.com is unreachable: {error}");
                    std::thread::sleep(time::Duration::from_secs(1));
                    continue
                },
            };

            match resp.body_mut().read_to_string() {
                Ok(text) => break text,
                Err(error) => {
                    println!("ERR");
                    eprintln!("Unable to get content of chapter: {}", error);
                }
            }
        };

        if let Err(error) = dump(&mut file, &text, &http_client) {
            println!("ERR");
            eprintln!("Failed to store novel. Error: {}", error);
            return ExitCode::FAILURE;
        }

        println!("OK");
    }

    let _ = io::Write::flush(&mut file);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let (is_stdin, args) = match cli::Cli::new() {
        Some(Ok(args)) => (false, args),
        Some(Err(code)) => return code,
        None => match args_from_stdin() {
            Ok(args) => (true, args),
            Err(code) => return code,
        }
    };

    let code = run(args);
    if is_stdin {
        let mut stdout = io::stdout();
        let _ = io::Write::write_all(&mut stdout, b"## Press ENTER to finish...");
        let _ = io::Write::flush(&mut stdout);
        let stdin = io::stdin();
        let mut buffer = String::new();
        let _ = stdin.read_line(&mut buffer);
        let _ = io::Write::write_all(&mut stdout, buffer.as_bytes());
        let _ = io::Write::flush(&mut stdout);
    }
    code
}

fn construct_file_path(dir: &str, name: &str) -> path::PathBuf {
    let mut path = path::PathBuf::from(dir);
    path.push(name);
    path.set_extension("md");

    path
}

fn dump<W: io::Write>(dest: &mut W, html: &str, http_client: &ureq::Agent) -> io::Result<()> {
    use kuchiki::traits::TendrilSink;

    const WHITE_SPACE: &[char] = &[' ', '\t', '\n', '　'];
    const NOVEL_BODY: &str = ".p-novel__text";
    const NOVEL_TITLE: &str = ".p-novel__title";

    let document = kuchiki::parse_html().from_utf8().one(html.as_bytes());

    let novel_title = match document.select_first(NOVEL_TITLE) {
        Ok(node) => node,
        Err(_) => return Err(io::Error::other("Unable to find .p-novel__title block")),
    };
    let novel_title = novel_title.as_node();

    let novel_text = match document.select_first(NOVEL_BODY) {
        Ok(node) => node,
        Err(_) => return Err(io::Error::other("Unable to find .p-novel__text block")),
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
                    let src = if src.starts_with("http") {
                        src.to_owned()
                    } else {
                        format!("https://{}", src.trim_start_matches('/'))
                    };
                    //Resolve indirection if any present
                    let src = match http_client.head(&src).call() {
                        Ok(resp) => match resp.status().as_u16() {
                            300..=399 => match resp.headers().get("location").and_then(|header| header.to_str().ok()) {
                                Some(header) => header.to_string(),
                                None => src,
                            }
                            _ => src,
                        }
                        Err(_) => src
                    };
                    let alt = img.get("alt").unwrap_or("");
                    dest.write_fmt(format_args!("![{alt}]({src})"))?;
                }
            }
        }
    }

    dest.write_all(b"\n\n")?;

    Ok(())
}
