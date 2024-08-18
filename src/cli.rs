use arg::Args;

use core::fmt;
use core::num::NonZeroUsize;
use std::process::ExitCode;

pub fn default_from_value() -> NonZeroUsize {
    unsafe {
        core::num::NonZeroUsize::new_unchecked(1)
    }
}

use crate::data::IdBuf;
#[derive(Debug)]
pub struct Id(pub IdBuf);

#[derive(Debug)]
pub struct IdOverflow(usize);

impl fmt::Display for IdOverflow {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("Id cannot be more than {} characters", self.0))
    }
}

impl core::str::FromStr for Id {
    type Err = IdOverflow;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() <= IdBuf::capacity() {
            let mut result = IdBuf::new();
            result.push_str(text);
            Ok(Id(result))
        } else {
            Err(IdOverflow(IdBuf::capacity()))
        }
    }
}

#[derive(Args, Debug)]
///Utility to download text of the syosetu novels
pub struct Cli {
    #[arg(long, default_value = "default_from_value()")]
    ///Specify from which chapter to start dumping. Default: 1.
    pub from: NonZeroUsize,
    #[arg(long)]
    ///Specify whether to access 18+ novel
    pub r18: bool,
    #[arg(long)]
    ///Specify until which chapter to dump.
    pub to: Option<NonZeroUsize>,
    #[arg(required)]
    ///Id of the novel to dump (e.g. n9185fm)
    pub novel: Id,
}

impl Cli {
    #[inline]
    pub fn new() -> Option<Result<Self, ExitCode>> {
        let args: std::vec::Vec<_> = std::env::args().skip(1).collect();

        if args.is_empty() {
            return None;
        }

        match Self::from_args(args.iter().map(std::string::String::as_str)) {
            Ok(args) => Some(Ok(args)),
            Err(arg::ParseKind::Sub(name, arg::ParseError::HelpRequested(help))) => {
                std::println!("{name}: {}", help);
                Some(Err(ExitCode::SUCCESS))
            },
            Err(arg::ParseKind::Top(arg::ParseError::HelpRequested(help))) => {
                std::println!("{}", help);
                Some(Err(ExitCode::SUCCESS))
            },
            Err(error) => {
                std::eprintln!("{}", error);
                Some(Err(ExitCode::FAILURE))
            }
        }
    }
}
