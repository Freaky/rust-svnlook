# rust-svnlook

A Rust crate for extracting information from Subversion repositories via the
`svnlook`command.

## Synopsis

```rust
let repo = svnlook::Svnlook::from("/path/to/repo");
let latest = repo.youngest()?;
for rev in 1..latest {
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
```