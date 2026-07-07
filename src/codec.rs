//! Pakkimis- ja lahtipakkimisvood: zstd ja xz.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};

use anyhow::{bail, Context, Result};

/// liblzma LZMA_PRESET_EXTREME lipp.
const LZMA_PRESET_EXTREME: u32 = 1 << 31;

/// zstd `--long` režiimile vastav akna suurus (128 MiB), mida ka tavaline
/// `zstd -d --long` ja `tar --zstd` oskavad lahti pakkida.
const ZSTD_WINDOW_LOG: u32 = 27;

#[derive(Clone, Copy, Debug)]
pub enum Profile {
    /// zstd tase 3 — kiireim.
    Fast,
    /// zstd tase 19 + long-distance matching — vaikimisi.
    Balanced,
    /// xz tase 9 + extreme — tihedaim.
    Max,
    /// Käsitsi valitud zstd tase.
    Zstd(i32),
}

impl Profile {
    pub fn describe(self) -> String {
        match self {
            Profile::Fast => "zstd-3 (kiire)".into(),
            Profile::Balanced => "zstd-19 + long (tasakaalus)".into(),
            Profile::Max => "xz-9e (tihedaim)".into(),
            Profile::Zstd(l) => format!("zstd-{l} + long"),
        }
    }
}

/// Pakkiv kirjutusvoog, mille saab eksplitsiitselt lõpetada.
pub enum Encoder {
    Zstd(zstd::stream::write::Encoder<'static, BufWriter<File>>),
    Xz(xz2::write::XzEncoder<BufWriter<File>>),
}

impl Encoder {
    /// Kirjutab pakitud voo lõpuploki ja tühjendab puhvrid kettale.
    pub fn finish(self) -> Result<()> {
        let mut out = match self {
            Encoder::Zstd(enc) => enc.finish().context("zstd voo lõpetamine ebaõnnestus")?,
            Encoder::Xz(enc) => enc.finish().context("xz voo lõpetamine ebaõnnestus")?,
        };
        out.flush().context("arhiivi kettale kirjutamine ebaõnnestus")?;
        Ok(())
    }
}

impl Write for Encoder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Encoder::Zstd(enc) => enc.write(buf),
            Encoder::Xz(enc) => enc.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Encoder::Zstd(enc) => enc.flush(),
            Encoder::Xz(enc) => enc.flush(),
        }
    }
}

/// Loob antud profiiliga pakkiva kirjutusvoo ümber väljundfaili.
pub fn encoder(file: File, profile: Profile, threads: usize) -> Result<Encoder> {
    let out = BufWriter::with_capacity(1 << 20, file);
    match profile {
        Profile::Max => {
            let stream = xz2::stream::MtStreamBuilder::new()
                .preset(9 | LZMA_PRESET_EXTREME)
                .threads(threads as u32)
                .check(xz2::stream::Check::Crc64)
                .encoder()
                .context("xz kodeerija loomine ebaõnnestus")?;
            Ok(Encoder::Xz(xz2::write::XzEncoder::new_stream(out, stream)))
        }
        Profile::Fast | Profile::Balanced | Profile::Zstd(_) => {
            let level = match profile {
                Profile::Fast => 3,
                Profile::Balanced => 19,
                Profile::Zstd(l) => l,
                Profile::Max => unreachable!(),
            };
            let mut enc = zstd::stream::write::Encoder::new(out, level)
                .context("zstd kodeerija loomine ebaõnnestus")?;
            enc.multithread(threads as u32)?;
            if !matches!(profile, Profile::Fast) {
                enc.long_distance_matching(true)?;
                enc.window_log(ZSTD_WINDOW_LOG)?;
            }
            Ok(Encoder::Zstd(enc))
        }
    }
}

/// Avab arhiivi ja tagastab lahtipakkiva lugemisvoo; vorming tuvastatakse
/// maagiliste baitide järgi.
pub fn decoder(file: File) -> Result<Box<dyn Read>> {
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let magic = peek(&mut reader)?;
    if magic.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        let mut dec = zstd::stream::read::Decoder::with_buffer(reader)
            .context("zstd dekoodri loomine ebaõnnestus")?;
        dec.window_log_max(31)?;
        Ok(Box::new(dec))
    } else if magic.starts_with(&[0xFD, b'7', b'z', b'X', b'Z', 0x00]) {
        Ok(Box::new(xz2::read::XzDecoder::new_multi_decoder(reader)))
    } else {
        bail!("tundmatu arhiivivorming: pole ei zstd ega xz voog");
    }
}

/// Piilub voo esimesi baite ilma lugemispositsiooni nihutamata.
fn peek(reader: &mut BufReader<File>) -> Result<Vec<u8>> {
    use std::io::BufRead;
    let buf = reader.fill_buf().context("arhiivi lugemine ebaõnnestus")?;
    if buf.len() < 6 {
        bail!("fail on arhiiviks liiga lühike");
    }
    Ok(buf[..6].to_vec())
}
