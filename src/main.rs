extern crate chrono;
use chrono::prelude::*;
use chrono::DateTime;

use std::ffi::OsStr;
use std::iter::Peekable;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::str::FromStr;

#[derive(Debug)]
struct SvnRepo {
    path: PathBuf,
}

#[derive(Debug)]
struct SvnInfo {
    revision: u32,
    committer: String,
    date: DateTime<FixedOffset>,
    message: String,
}

#[derive(Debug)]
enum SvnStatus {
    Added,
    Copied,
    Deleted,
    Updated,
    PropChange,
    Other,
}

impl FromStr for SvnStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "A   " => SvnStatus::Added,
            "A + " => SvnStatus::Copied,
            "D   " => SvnStatus::Deleted,
            "U   " => SvnStatus::Updated,
            "_U  " => SvnStatus::PropChange,
            "UU  " => SvnStatus::Updated,
            _ => SvnStatus::Other,
        })
    }
}

#[derive(Debug)]
struct Delta {
    additions: u32,
    deletions: u32,
}

#[derive(Debug)]
struct SvnFrom {
    path: PathBuf,
    revision: u32,
}

#[derive(Debug)]
struct SvnChange {
    path: PathBuf,
    status: SvnStatus,
    from: Option<SvnFrom>,
    delta: Option<Delta>,
}

impl SvnRepo {
    fn new<R: AsRef<Path>>(path: R) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    fn youngest(&self) -> u32 {
        let n = Command::new("svnlook")
            .arg("youngest")
            .arg(&self.path)
            .output()
            .expect("svnlook");

        str::from_utf8(&n.stdout[..])
            .expect("utf8")
            .trim()
            .parse()
            .expect("number")
    }

    fn info(&self, revision: u32) -> SvnInfo {
        let n = Command::new("svnlook")
            .arg("info")
            .arg("-r")
            .arg(revision.to_string())
            .arg(&self.path)
            .output()
            .expect("svnlook");

        let mut o = n.stdout.splitn(4, |b| *b == b'\n');
        let committer = String::from_utf8_lossy(o.next().expect("committer")).to_string();
        let date = DateTime::parse_from_str(
            str::from_utf8(&o.next().expect("date")[0..25]).expect("date"),
            "%Y-%m-%d %H:%M:%S %z",
        )
        .expect("date");
        let bytes = str::from_utf8(o.next().expect("message size 1"))
            .expect("message size 2")
            .parse::<usize>()
            .expect("message size 3");
        let message = String::from_utf8_lossy(&o.next().expect("message")[0..bytes]).to_string();

        SvnInfo {
            revision,
            committer,
            date,
            message,
        }
    }

    // iterator?
    fn changed(&self, revision: u32) -> Vec<SvnChange> {
        let n = Command::new("svnlook")
            .arg("--copy-info")
            .arg("changed")
            .arg("-r")
            .arg(revision.to_string())
            .arg(&self.path)
            .output()
            .expect("svnlook");

        let mut lines = n
            .stdout
            .split(|&b| b == b'\n')
            .filter(|s| s.len() > 4)
            .peekable();
        let mut changes = vec![];

        while let Some(line) = lines.next() {
            let (change, path) = line.split_at(4);
            let mut change = SvnChange {
                path: PathBuf::from(OsStr::from_bytes(path)),
                status: str::from_utf8(change)
                    .ok()
                    .and_then(|s| SvnStatus::from_str(s).ok())
                    .unwrap_or(SvnStatus::Other),
                from: None,
                delta: None,
            };

            change.from = lines
                .peek()
                .filter(|line| line.starts_with(b"    (from ") && line.ends_with(b")"))
                .map(|line| &line[10..line.len() - 1])
                .and_then(|line| {
                    line.iter()
                        .rposition(|&b| b == b':')
                        .map(|pos| line.split_at(pos))
                })
                .filter(|(_path, revision)| revision.len() > 2)
                .map(|(path, revision)| SvnFrom {
                    path: PathBuf::from(OsStr::from_bytes(path)),
                    revision: str::from_utf8(&revision[2..])
                        .ok()
                        .and_then(|s| u32::from_str(s).ok())
                        .unwrap_or(0),
                });

            if change.from.is_some() {
                lines.next();
            }

            changes.push(change);
        }

        changes
    }

    // io::Read?
    fn diff<R: AsRef<Path>>(&self, revision: u32, filename: Option<R>) -> String {
        unimplemented!()
    }

    // io::Read?
    fn cat(&self, revision: u32, filename: PathBuf, limit: Option<usize>) -> String {
        unimplemented!()
    }

    // iterator?
    fn diffstat(&self, revision: u32) {}
}

fn main() {
    let repo = SvnRepo::new("/repos/freebsd");

    let latest = repo.youngest();

    // for rev in (latest-100)..latest {
    for rev in 1..latest {
        println!("{:?}", repo.info(rev));
        println!("{:?}", repo.changed(rev));
    }
}
