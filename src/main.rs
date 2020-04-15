use svnlook::{SvnRepo, SvnStatus};

use std::env;

fn main() -> Result<(), svnlook::SvnError> {
    let cmd = env::args().nth(1).expect("Need a command");
    let repo = SvnRepo::from(env::args_os().nth(2).expect("Need a repository path"));

    match &cmd[..] {
        "youngest" => println!("{}", repo.youngest()?),
        "changes" => {
            let rev = env::args()
                .nth(3)
                .expect("Need a revision")
                .parse()
                .expect("Not a number");
            for change in repo.changed(rev)? {
                let change = change?;
                print!("   {:.8}: ", change.status);

                if let SvnStatus::Copied(from) = change.status {
                    print!("{}@r{} -> ", from.path.display(), from.revision);
                }

                println!("{}", change.path.display());
            }
        }
        "diff" => {
            let rev = env::args()
                .nth(3)
                .expect("Need a revision")
                .parse()
                .expect("Not a number");
            std::io::copy(&mut repo.diff(rev)?, &mut std::io::stdout())?;
        }
        "cat" => {
            let rev = env::args()
                .nth(3)
                .expect("Need a revision")
                .parse()
                .expect("Not a number");
            let path = env::args().nth(4).expect("Need a file path");
            std::io::copy(&mut repo.cat(rev, path)?, &mut std::io::stdout())?;
        }
        "walk" => {
            let from = env::args()
                .nth(3)
                .map(|s| s.parse().expect("Not a number"))
                .unwrap_or(1);
            let latest = repo.youngest()?;

            for rev in from..latest {
                let info = repo.info(rev)?;
                let changed = repo.changed(rev)?;

                println!(
                    "Revision r{}, by {} at {}",
                    info.revision, info.committer, info.date
                );
                for change in changed {
                    let change = change?;
                    print!("   {:.8}: ", change.status);

                    if let SvnStatus::Copied(from) = change.status {
                        print!("{}@r{} -> ", from.path.display(), from.revision);
                    }

                    println!("{}", change.path.display());
                }
            }
        }
        _ => {
            panic!("Commands: youngest, changes, diff, cat, walk");
        }
    }

    Ok(())
}
