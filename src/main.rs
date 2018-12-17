
extern crate svnlook;
use svnlook::SvnRepo;

use std::env;

fn main() {
    let repo = SvnRepo::new(env::args_os().nth(1).expect("Need a path"));

    let latest = repo.youngest().expect("youngest");

    for rev in 1..latest {
        let info = repo.info(rev).expect("info");
        let changed = repo.changed(rev).expect("changed");

        println!("Revision r{}, by {} at {}", info.revision, info.committer, info.date);
        for change in changed {
            print!("   {:.8}: ", change.status);

            if let Some(from) = change.from {
                println!("{}@r{} -> {}", from.path.display(), from.revision, change.path.display());
            } else {
                println!("{}", change.path.display());
            }
        }
    }
}
