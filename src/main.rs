extern crate chrono;
use chrono::prelude::*;
use chrono::DateTime;

use std::ffi::OsStr;
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
struct SvnChange {
    path: PathBuf,
    status: SvnStatus,
    old_path: Option<PathBuf>,
    old_revision: Option<u32>,
    additions: Option<u32>,
    deletions: Option<u32>,
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

        n.stdout
            .split(|b| *b == b'\n')
            .filter(|str| str.len() > 4)
            .map(|str| str.split_at(4))
            .map(|(change, path)| SvnChange {
                path: PathBuf::from(OsStr::from_bytes(path)),
                status: SvnStatus::from_str(str::from_utf8(change).unwrap()).unwrap(),
                old_path: None,
                old_revision: None,
                additions: None,
                deletions: None,
            })
            .collect::<Vec<SvnChange>>()
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
