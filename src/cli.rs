use arg::Args;

use core::fmt;
use core::num::NonZeroUsize;

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
    #[arg(long, default_value = "unsafe { core::num::NonZeroUsize::new_unchecked(1) }")]
    ///Specify from which chapter to start dumping. Default: 1.
    pub from: NonZeroUsize,
    #[arg(long)]
    ///Specify until which chapter to dump.
    pub to: Option<NonZeroUsize>,
    #[arg(required)]
    ///Id of the novel to dump (e.g. n9185fm)
    pub novel: Id,
}

impl Cli {
    #[inline]
    pub fn new<'a, T: IntoIterator<Item = &'a str>>(args: T) -> Result<Self, bool> {
        let args = args.into_iter();

        Cli::from_args(args).map_err(|err| match err.is_help() {
            true => {
                println!("{}", Cli::HELP);
                false
            },
            false => {
                eprintln!("{}", err);
                true
            },
        })
    }
}
