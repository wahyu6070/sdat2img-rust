//! Rust port of sdat2img (https://github.com/xpirt/sdat2img).
//!
//! Converts an Android sparse data image (`*.new.dat` + `*.transfer.list`)
//! into a raw filesystem image (`*.img`).

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const BLOCK_SIZE: u64 = 4096;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{}", e),
            Error::Parse(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Erase,
    New,
    Zero,
}

impl fmt::Display for CommandKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CommandKind::Erase => "erase",
            CommandKind::New => "new",
            CommandKind::Zero => "zero",
        })
    }
}

/// Half-open block range `[begin, end)`.
pub type Range = (u64, u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub kind: CommandKind,
    pub ranges: Vec<Range>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferList {
    pub version: u32,
    pub new_blocks: u64,
    pub commands: Vec<Command>,
}

impl TransferList {
    /// Size in bytes the output image must have: the highest block touched
    /// by any command, times the block size.
    pub fn max_file_size(&self) -> Result<u64> {
        let max_block = self
            .commands
            .iter()
            .flat_map(|c| c.ranges.iter())
            .map(|&(_, end)| end)
            .max()
            .unwrap_or(0);
        block_offset(max_block)
    }
}

pub fn android_version_name(version: u32) -> &'static str {
    match version {
        1 => "Android Lollipop 5.0 detected!",
        2 => "Android Lollipop 5.1 detected!",
        3 => "Android Marshmallow 6.x detected!",
        4 => "Android Nougat 7.x / Oreo 8.x detected!",
        _ => "Unknown Android version!",
    }
}

fn block_offset(block: u64) -> Result<u64> {
    block
        .checked_mul(BLOCK_SIZE)
        .ok_or_else(|| Error::Parse(format!("Block number {} is too large", block)))
}

fn parse_number<T: std::str::FromStr>(s: &str, what: &str) -> Result<T> {
    s.trim()
        .parse()
        .map_err(|_| Error::Parse(format!("Invalid {}: \"{}\"", what, s.trim())))
}

/// Parses a rangeset string such as `4,0,10,20,30` into `[(0,10), (20,30)]`.
/// The first number is the count of numbers that follow.
pub fn parse_rangeset(src: &str) -> Result<Vec<Range>> {
    let bad = || {
        Error::Parse(format!(
            "Error on parsing following data to rangeset:\n{}",
            src
        ))
    };

    let nums = src
        .trim()
        .split(',')
        .map(|s| s.trim().parse::<u64>().map_err(|_| bad()))
        .collect::<Result<Vec<u64>>>()?;

    let count = *nums.first().ok_or_else(bad)?;
    if nums.len() as u64 != count + 1 || count % 2 != 0 {
        return Err(bad());
    }

    let ranges: Vec<Range> = nums[1..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[b, e]| (b, e))
        .collect();
    if ranges.iter().any(|&(begin, end)| begin > end) {
        return Err(bad());
    }
    Ok(ranges)
}

/// Parses a transfer list from any buffered reader.
pub fn parse_transfer_list<R: BufRead>(reader: R) -> Result<TransferList> {
    let mut lines = reader.lines();
    let mut next_header = |what: &str| -> Result<String> {
        lines
            .next()
            .transpose()?
            .ok_or_else(|| Error::Parse(format!("Transfer list is missing the {} line", what)))
    };

    // First line in transfer list is the version number
    let version: u32 = parse_number(&next_header("version")?, "version")?;

    // Second line in transfer list is the total number of blocks we expect to write
    let new_blocks: u64 = parse_number(&next_header("total blocks")?, "total blocks")?;

    if version >= 2 {
        // Third line is how many stash entries are needed simultaneously
        next_header("stash entries")?;
        // Fourth line is the maximum number of blocks that will be stashed simultaneously
        next_header("max stash blocks")?;
    }

    // Subsequent lines are all individual transfer commands
    let mut commands = Vec::new();
    for line in lines {
        let line = line?;
        let mut parts = line.split_whitespace();
        let cmd = match parts.next() {
            Some(c) => c,
            None => continue,
        };

        let kind = match cmd {
            "erase" => CommandKind::Erase,
            "new" => CommandKind::New,
            "zero" => CommandKind::Zero,
            // Skip lines starting with numbers, they are not commands anyway
            _ if cmd.starts_with(|c: char| c.is_ascii_digit()) => continue,
            _ => {
                return Err(Error::Parse(format!(
                    "Command \"{}\" is not valid.\n\
                     Only full OTA transfer lists (erase/new/zero) are supported.",
                    cmd
                )))
            }
        };

        let arg = parts
            .next()
            .ok_or_else(|| Error::Parse(format!("Command \"{}\" has no rangeset", cmd)))?;
        commands.push(Command {
            kind,
            ranges: parse_rangeset(arg)?,
        });
    }

    Ok(TransferList {
        version,
        new_blocks,
        commands,
    })
}

pub fn parse_transfer_list_file<P: AsRef<Path>>(path: P) -> Result<TransferList> {
    parse_transfer_list(BufReader::new(File::open(path)?))
}

/// Writes the image described by `list` to `output`, taking block data
/// sequentially from `new_data`. `log` receives progress messages.
pub fn write_image<R, W>(
    list: &TransferList,
    new_data: &mut R,
    output: &mut W,
    mut log: impl FnMut(&str),
) -> Result<()>
where
    R: Read,
    W: Write + Seek,
{
    for command in &list.commands {
        if command.kind != CommandKind::New {
            log(&format!("Skipping command {}...", command.kind));
            continue;
        }

        for &(begin, end) in &command.ranges {
            let block_count = end - begin;
            log(&format!(
                "Copying {} blocks into position {}...",
                block_count, begin
            ));

            // Position output file
            output.seek(SeekFrom::Start(block_offset(begin)?))?;

            let len = block_offset(block_count)?;
            let copied = io::copy(&mut new_data.by_ref().take(len), output)?;
            if copied != len {
                return Err(Error::Parse(format!(
                    "Unexpected end of new data file: needed {} bytes for blocks {}..{}, got {}",
                    len, begin, end, copied
                )));
            }
        }
    }
    output.flush()?;
    Ok(())
}

/// Full conversion: parsed transfer list + new.dat -> image file.
pub fn convert<Q, S>(
    list: &TransferList,
    new_data: Q,
    output: S,
    log: impl FnMut(&str),
) -> Result<()>
where
    Q: AsRef<Path>,
    S: AsRef<Path>,
{
    let max_file_size = list.max_file_size()?;

    let mut new_data = BufReader::with_capacity(1 << 20, File::open(new_data)?);
    let mut output_img = File::create(output)?;

    write_image(list, &mut new_data, &mut output_img, log)?;

    // Make file larger if necessary
    if output_img.metadata()?.len() < max_file_size {
        output_img.set_len(max_file_size)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rangeset_ok() {
        assert_eq!(parse_rangeset("2,0,10").unwrap(), vec![(0, 10)]);
        assert_eq!(
            parse_rangeset("4,0,10,20,30\r\n").unwrap(),
            vec![(0, 10), (20, 30)]
        );
    }

    #[test]
    fn rangeset_bad() {
        assert!(parse_rangeset("3,0,10").is_err());
        assert!(parse_rangeset("3,0,10,20").is_err());
        assert!(parse_rangeset("2,10,0").is_err());
        assert!(parse_rangeset("x").is_err());
        assert!(parse_rangeset("").is_err());
    }

    #[test]
    fn transfer_list_v4() {
        let src = "4\n3\n0\n0\nerase 2,0,8\nnew 4,2,3,0,2\n\nzero 2,3,8\n";
        let list = parse_transfer_list(Cursor::new(src)).unwrap();
        assert_eq!(list.version, 4);
        assert_eq!(list.new_blocks, 3);
        assert_eq!(list.commands.len(), 3);
        assert_eq!(list.commands[1].ranges, vec![(2, 3), (0, 2)]);
        assert_eq!(list.max_file_size().unwrap(), 8 * BLOCK_SIZE);
    }

    #[test]
    fn transfer_list_rejects_diff_commands() {
        let src = "4\n1\n0\n0\nmove 2,0,1 2,1,2\n";
        assert!(parse_transfer_list(Cursor::new(src)).is_err());
    }

    #[test]
    fn transfer_list_skips_numeric_lines() {
        let src = "1\n1\nnew 2,0,1\n123 junk\n";
        let list = parse_transfer_list(Cursor::new(src)).unwrap();
        assert_eq!(list.commands.len(), 1);
    }

    #[test]
    fn write_places_blocks() {
        let list = parse_transfer_list(Cursor::new("1\n3\nnew 4,2,3,0,2\n")).unwrap();
        let data: Vec<u8> = (0..3u8)
            .flat_map(|b| std::iter::repeat_n(b + 1, BLOCK_SIZE as usize))
            .collect();
        let mut out = Cursor::new(Vec::new());
        write_image(&list, &mut Cursor::new(data), &mut out, |_| {}).unwrap();
        let out = out.into_inner();
        let bs = BLOCK_SIZE as usize;
        assert_eq!(out.len(), 3 * bs);
        assert!(out[2 * bs..].iter().all(|&b| b == 1));
        assert!(out[..bs].iter().all(|&b| b == 2));
        assert!(out[bs..2 * bs].iter().all(|&b| b == 3));
    }

    #[test]
    fn write_errors_on_short_data() {
        let list = parse_transfer_list(Cursor::new("1\n2\nnew 2,0,2\n")).unwrap();
        let data = vec![0u8; BLOCK_SIZE as usize];
        let mut out = Cursor::new(Vec::new());
        assert!(write_image(&list, &mut Cursor::new(data), &mut out, |_| {}).is_err());
    }
}
