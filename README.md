# NAME

`rust-svnlook` - a Rusty interface to the `svnlook` command

## SYNOPSIS

```rust
// Equivalent to svnlook::Svnlook::from("svnlook").repository("/path/to/repo");
let repo = svnlook::Repository::from("/path/to/repo");
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

        if let svnlook::SvnStatus::Copied(from) = change.status {
            print!("{}@r{} -> ", from.path.display(), from.revision);
        }

        println!("{}", change.path.display());

        println!("File contents:");
        println!("==============================");
        std::io::copy(&mut repo.cat(rev, change.path)?, std::io::stdout())?;
        println!("==============================");
    }

    println!("Revision diff:");
    println!("==============================");
    std::io::copy(&mut repo.diff().revision(rev).spawn()?, std::io::stdout())?;
    println!("==============================");
}
```

## DESCRIPTION

`rust-svnlook` provides a (hopefully) robust, typed, and convenient interface
to examining a local Subversion repository using the `svnlook` command.

The `changed` command offers a streaming iterator, converting lines read from
svnlook into structs.  `diff` and `cat` provide a streaming `BufRead`
implementation.

Both check the command exits successfully on EOF to minimise the risk of missing
an erroring command, and can be dropped safely at any point: unlike `Command`,
their `Drop` implementation will reap the child process and silently swallow
any error.

## SEE ALSO

* [Apache Subversion](https://subversion.apache.org/)
* [svnlook](http://svnbook.red-bean.com/en/1.7/svn.ref.svnlook.html)
* At least it isn't [CVS](https://www.nhs.uk/conditions/cyclical-vomiting-syndrome/)

## AUTHORS

Thomas Hurst <tom@hur.st>
