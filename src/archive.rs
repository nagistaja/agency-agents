//! Arhiivi loomine, lahtipakkimine ja sisu näitamine (tar-konteiner).

use std::fs::File;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::codec::{self, Profile};
use crate::stats;

/// Loob arhiivi: tar-voog läbi valitud pakkimisprofiili.
pub fn create(
    archive: &Path,
    inputs: &[PathBuf],
    profile: Profile,
    threads: usize,
    verbose: bool,
) -> Result<()> {
    for input in inputs {
        if !input.exists() {
            bail!("sisendit ei leitud: {}", input.display());
        }
    }

    let started = Instant::now();
    let file = File::create(archive)
        .with_context(|| format!("ei saa luua arhiivi {}", archive.display()))?;
    let mut builder = tar::Builder::new(codec::encoder(file, profile, threads)?);
    builder.follow_symlinks(false);

    let mut original: u64 = 0;
    for input in inputs {
        let name = entry_name(input)?;
        original += add_path(&mut builder, input, &name, verbose)?;
    }

    builder
        .into_inner()
        .context("tar-voo lõpetamine ebaõnnestus")?
        .finish()?;

    let compressed = std::fs::metadata(archive)?.len();
    let elapsed = started.elapsed();
    println!(
        "valmis: {}  |  {}  |  {} -> {}  ({:.1}% algsest)  |  {:.1}s, {}/s",
        archive.display(),
        profile.describe(),
        stats::human(original),
        stats::human(compressed),
        if original > 0 { compressed as f64 / original as f64 * 100.0 } else { 0.0 },
        elapsed.as_secs_f64(),
        stats::human((original as f64 / elapsed.as_secs_f64().max(0.001)) as u64),
    );
    Ok(())
}

/// Lisab faili või kataloogi (rekursiivselt) tar-voogu; tagastab lisatud
/// regulaarfailide kogumahu.
fn add_path<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    path: &Path,
    name: &Path,
    verbose: bool,
) -> Result<u64> {
    let meta = std::fs::symlink_metadata(path)
        .with_context(|| format!("ei saa lugeda: {}", path.display()))?;
    if meta.is_dir() {
        let mut total = 0;
        builder
            .append_path_with_name(path, name)
            .with_context(|| format!("ei saa lisada kataloogi: {}", path.display()))?;
        let mut entries: Vec<_> = std::fs::read_dir(path)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let child = entry.path();
            let child_name = name.join(entry.file_name());
            total += add_path(builder, &child, &child_name, verbose)?;
        }
        Ok(total)
    } else {
        if verbose {
            println!("+ {}", name.display());
        }
        builder
            .append_path_with_name(path, name)
            .with_context(|| format!("ei saa lisada faili: {}", path.display()))?;
        Ok(if meta.is_file() { meta.len() } else { 0 })
    }
}

/// Teisendab sisenditee arhiivisiseseks nimeks: absoluutteelt eemaldatakse
/// juurikas ning `..`/`.` komponendid jäetakse välja.
fn entry_name(path: &Path) -> Result<PathBuf> {
    let mut name = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Normal(part) => name.push(part),
            Component::RootDir | Component::Prefix(_) | Component::CurDir => {}
            Component::ParentDir => {}
        }
    }
    if name.as_os_str().is_empty() {
        // nt sisend "." või "/" — kasutame kataloogi tegelikku nime
        let canonical = path.canonicalize()?;
        match canonical.file_name() {
            Some(part) => name.push(part),
            None => name.push("juur"),
        }
    }
    Ok(name)
}

/// Pakib arhiivi lahti sihtkataloogi.
pub fn extract(archive: &Path, dir: &Path) -> Result<()> {
    let started = Instant::now();
    let file = File::open(archive)
        .with_context(|| format!("ei saa avada arhiivi {}", archive.display()))?;
    std::fs::create_dir_all(dir)
        .with_context(|| format!("ei saa luua sihtkataloogi {}", dir.display()))?;

    let mut reader = tar::Archive::new(codec::decoder(file)?);
    reader.set_preserve_permissions(true);
    reader.set_preserve_mtime(true);
    let mut count = 0usize;
    for entry in reader.entries()? {
        let mut entry = entry?;
        entry.unpack_in(dir).with_context(|| {
            format!("kirje lahtipakkimine ebaõnnestus: {:?}", entry.path())
        })?;
        count += 1;
    }

    println!(
        "lahti pakitud: {} kirjet -> {}  ({:.1}s)",
        count,
        dir.display(),
        started.elapsed().as_secs_f64(),
    );
    Ok(())
}

/// Näitab arhiivi sisu ilma lahti pakkimata.
pub fn list(archive: &Path) -> Result<()> {
    let file = File::open(archive)
        .with_context(|| format!("ei saa avada arhiivi {}", archive.display()))?;
    let mut reader = tar::Archive::new(codec::decoder(file)?);
    let mut count = 0usize;
    let mut total = 0u64;
    for entry in reader.entries()? {
        let entry = entry?;
        let size = entry.header().size().unwrap_or(0);
        println!("{:>12}  {}", stats::human(size), entry.path()?.display());
        total += size;
        count += 1;
    }
    println!("kokku: {count} kirjet, {}", stats::human(total));
    Ok(())
}
