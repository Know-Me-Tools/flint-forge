//! Read one approved subject and tenant, one UTF-8 line each, from stdin.
//! Write the corresponding static Cedar policy to stdout.
use std::io::{self, BufRead, Write};

fn main() -> io::Result<()> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected subject and tenant on two nonempty lines",
        )
    };
    let mut lines = io::stdin().lock().lines();
    let subject = lines.next().transpose()?.ok_or_else(invalid)?;
    let tenant = lines.next().transpose()?.ok_or_else(invalid)?;
    if lines.next().transpose()?.is_some() {
        return Err(invalid());
    }
    let policy = forge_policy::files::record_grant(&subject, &tenant).ok_or_else(invalid)?;
    io::stdout().lock().write_all(policy.as_bytes())
}
