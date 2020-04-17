use std::env;

fn worlds_crappiest_diffstat<R: std::io::BufRead>(diff: R) -> std::io::Result<(u32, u32)> {
    let mut counts = (0, 0);
    for line in diff.split(b'\n') {
        match line?.first() {
            Some(b'+') => {
                counts.0 += 1;
            }
            Some(b'-') => {
                counts.1 += 1;
            }
            _ => (),
        }
    }
    Ok(counts)
}

fn main() -> Result<(), svnlook::SvnError> {
    let cmd = env::args().nth(1).expect("Need a command");
    let repo = svnlook::Repository::from(env::args_os().nth(2).expect("Need a repository path"));

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

                if let svnlook::SvnStatus::Copied(from) = change.status {
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
            std::io::copy(
                &mut repo.diff().revision(rev).diff_copy_from().spawn()?,
                &mut std::io::stdout(),
            )?;
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
                let diff = repo.diff().revision(rev).spawn()?;

                println!(
                    "Revision r{}, by {} at {}",
                    info.revision, info.committer, info.date
                );
                for change in changed {
                    let change = change?;
                    print!("   {:.8}: ", change.status);

                    if let svnlook::SvnStatus::Copied(from) = change.status {
                        print!("{}@r{} -> ", from.path.display(), from.revision);
                    }

                    println!("{}", change.path.display());
                }

                let diff = worlds_crappiest_diffstat(diff)?;
                println!("Delta: +{} -{}", diff.0, diff.1);
            }
        }
        _ => {
            panic!("Commands: youngest, changes, diff, cat, walk");
        }
    }

    Ok(())
}
