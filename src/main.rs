//! pakk — kiire ja tõhus mitmelõimeline pakkimistööriist.
//!
//! Arhiivivorming on standardne tar-voog, mis on pakitud kas zstd või xz
//! algoritmiga, seega on failid avatavad ka tavaliste tööriistadega
//! (`tar --zstd -xf`, `tar -Jxf`).

mod archive;
mod codec;
mod stats;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pakk",
    version,
    about = "Kiire ja tõhus pakkimistööriist (fast, high-ratio archiver)",
    after_help = "Näited / examples:\n  \
        pakk c projekt.pakk src/ docs/        # tasakaalus (zstd-19, long)\n  \
        pakk c --fast logid.pakk /var/log     # kiireim (zstd-3)\n  \
        pakk c --max varukoopia.pakk andmed/  # tihedaim (xz-9e)\n  \
        pakk x projekt.pakk -C /tmp/valjund   # paki lahti\n  \
        pakk l projekt.pakk                   # näita sisu"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Loo arhiiv (create)
    #[command(visible_alias = "create")]
    C {
        /// Loodav arhiivifail, nt projekt.pakk
        archive: PathBuf,
        /// Failid ja kataloogid, mis arhiivi lisada
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Kiireim režiim: zstd tase 3
        #[arg(long, conflicts_with_all = ["max", "level"])]
        fast: bool,
        /// Tihedaim režiim: xz tase 9 + extreme
        #[arg(long, conflicts_with_all = ["fast", "level"])]
        max: bool,
        /// Käsitsi zstd tase (1–22)
        #[arg(short, long, value_parser = clap::value_parser!(i32).range(1..=22))]
        level: Option<i32>,
        /// Lõimede arv (vaikimisi kõik tuumad)
        #[arg(short, long)]
        threads: Option<usize>,
        /// Näita iga lisatavat faili
        #[arg(short, long)]
        verbose: bool,
    },
    /// Paki arhiiv lahti (extract)
    #[command(visible_alias = "extract")]
    X {
        /// Lahtipakitav arhiiv
        archive: PathBuf,
        /// Sihtkataloog
        #[arg(short = 'C', long = "dir", default_value = ".")]
        dir: PathBuf,
    },
    /// Näita arhiivi sisu (list)
    #[command(visible_alias = "list")]
    L {
        /// Arhiiv, mille sisu näidata
        archive: PathBuf,
    },
}

fn main() -> Result<()> {
    // Taasta SIGPIPE vaikimisi käitumine, et `pakk l arhiiv | head` ei
    // lõpeks paanikaga.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::C { archive, inputs, fast, max, level, threads, verbose } => {
            let threads = threads.unwrap_or_else(num_cpus::get).max(1);
            let profile = if fast {
                codec::Profile::Fast
            } else if max {
                codec::Profile::Max
            } else if let Some(l) = level {
                codec::Profile::Zstd(l)
            } else {
                codec::Profile::Balanced
            };
            archive::create(&archive, &inputs, profile, threads, verbose)
        }
        Cmd::X { archive, dir } => archive::extract(&archive, &dir),
        Cmd::L { archive } => archive::list(&archive),
    }
}
