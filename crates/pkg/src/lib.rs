use std::io;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use walkdir::WalkDir;
use zip::write::FileOptions;

pub fn build_zip(source: &Utf8Path, output: &Utf8Path) -> Result<()> {
    if !source.exists() {
        anyhow::bail!("source {} missing", source);
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(output).with_context(|| format!("creating {output}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(source).unwrap();
        let rel = Utf8PathBuf::from(rel.to_string_lossy().to_string());
        if entry.file_type().is_dir() {
            if !rel.as_str().is_empty() {
                zip.add_directory(rel.as_str(), options)?;
            }
            continue;
        }
        let mut f = fs::File::open(path)?;
        zip.start_file(rel.as_str(), options)?;
        io::copy(&mut f, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}
