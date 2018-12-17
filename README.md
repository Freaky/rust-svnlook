# rust-svnlook

A Rust crate for extracting information from Subversion repositories via the
`svnlook` command.

## Synopsis

```rust
repo = SvnRepo::new("/path/to/repo");

let rev = repo.youngest().expect("youngest");
let info = repo.info(rev).expect("info");
let changed = repo.changed(rev).expect("changed");

println!("Revision r{}, by {} at {}", info.revision, info.committer, info.date);
for change in changed {
    print!("   {:.8}: ", change.status);

    if let Some(from) = change.from {
        print!("{}@r{} -> ", from.path.display(), from.revision);
    }

    println!("{}", change.path.display());
}
```