use svnlook::SvnRepo;

use std::env;

fn main() -> Result<(), svnlook::SvnError> {
    let repo = SvnRepo::new(env::args_os().nth(1).expect("Need a path"));

    let latest = repo.youngest()?;

    for rev in 1..latest {
        let info = repo.info(rev)?;
        let changed = repo.changed(rev)?;

        println!(
            "Revision r{}, by {} at {}",
            info.revision, info.committer, info.date
        );
        for change in changed {
            print!("   {:.8}: ", change.status);

            if let Some(from) = change.from {
                print!("{}@r{} -> ", from.path.display(), from.revision);
            }

            println!("{}", change.path.display());
        }
    }

    Ok(())
}
