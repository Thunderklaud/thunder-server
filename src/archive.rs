use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use actix_web::http::header::ContentEncoding;
use libflate::gzip::Encoder;
use serde::Deserialize;
use strum::{Display, EnumIter, EnumString};
use tar::Builder;
use zip::{write, ZipWriter};

/// Available archive methods
#[derive(Deserialize, Clone, Copy, EnumIter, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ArchiveMethod {
    TarGz,
    Tar,
    Zip,
}

impl ArchiveMethod {
    pub fn extension(self) -> String {
        match self {
            ArchiveMethod::TarGz => "tar.gz",
            ArchiveMethod::Tar => "tar",
            ArchiveMethod::Zip => "zip",
        }
        .to_string()
    }

    pub fn content_type(self) -> String {
        match self {
            ArchiveMethod::TarGz => "application/gzip",
            ArchiveMethod::Tar => "application/tar",
            ArchiveMethod::Zip => "application/zip",
        }
        .to_string()
    }

    pub fn content_encoding(self) -> ContentEncoding {
        match self {
            ArchiveMethod::TarGz => ContentEncoding::Gzip,
            ArchiveMethod::Tar => ContentEncoding::Identity,
            ArchiveMethod::Zip => ContentEncoding::Identity,
        }
    }

    pub fn is_enabled(self, tar_enabled: bool, tar_gz_enabled: bool, zip_enabled: bool) -> bool {
        match self {
            ArchiveMethod::TarGz => tar_gz_enabled,
            ArchiveMethod::Tar => tar_enabled,
            ArchiveMethod::Zip => zip_enabled,
        }
    }

    /// Make an archive out of the given Vec<FileWithPath>, and write the output to the given writer.
    pub fn create_archive<W>(self, files: Vec<FileWithPath>, out: W) -> actix_web::Result<()>
    where
        W: std::io::Write,
    {
        match self {
            ArchiveMethod::TarGz => tar_gz(files, out),
            ArchiveMethod::Tar => tar(files, out),
            ArchiveMethod::Zip => zip_data(files, out),
        }
    }
}

/// Write a gzipped tarball of `files` in `out`.
fn tar_gz<W>(files: Vec<FileWithPath>, out: W) -> actix_web::Result<()>
where
    W: std::io::Write,
{
    let mut out = Encoder::new(out)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("GZIP, {:?}", e)))?;

    tar(files, &mut out)?;

    out.finish()
        .into_result()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("GZIP finish, {:?}", e)))?;

    Ok(())
}

pub struct FileWithPath {
    pub file: File,
    pub path: String,
}

/// Writes a tarball of `files` in `out`.
fn tar<W>(files: Vec<FileWithPath>, out: W) -> actix_web::Result<()>
where
    W: std::io::Write,
{
    let mut tar_builder = Builder::new(out);

    for mut fp in files {
        // Adds the defined files into the archive stream
        tar_builder
            .append_file(Path::new(&fp.path), &mut fp.file)
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to append the content of {} to the TAR archive {:?}",
                    fp.path, e
                ))
            })?;
    }

    // Finish the archive
    tar_builder.into_inner().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to finish writing the TAR archive, {:?}",
            e
        ))
    })?;

    Ok(())
}

/// Writes a zip of `files` in `out`.
fn zip_data<W>(files: Vec<FileWithPath>, mut out: W) -> actix_web::Result<()>
where
    W: std::io::Write,
{
    let mut data = Vec::new();
    let memory_file = Cursor::new(&mut data);

    create_zip_from_file_with_path_vec(memory_file, files).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to create the ZIP archive, {:?}",
            e
        ))
    })?;

    out.write_all(data.as_mut_slice()).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to write the ZIP archive, {:?}",
            e
        ))
    })?;

    Ok(())
}

/// Write a zip of `dir` in `out`.
///
/// The target directory will be saved as a top-level directory in the archive.
///
/// For example, consider this directory structure:
///
/// ```ignore
/// a
/// └── b
///     └── c
///         ├── e
///         ├── f
///         └── g
/// ```
///
/// Making a zip out of `"a/b/c"` will result in this archive content:
///
/// ```ignore
/// c
/// ├── e
/// ├── f
/// └── g
/// ```
fn create_zip_from_file_with_path_vec<W>(out: W, files: Vec<FileWithPath>) -> actix_web::Result<()>
where
    W: std::io::Write + std::io::Seek,
{
    let options = write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    /*let mut paths_queue: Vec<PathBuf> = vec![directory.to_path_buf()];
    let zip_root_folder_name = directory.file_name().ok_or_else(|| {
        ContextualError::InvalidPathError("Directory name terminates in \"..\"".to_string())
    })?;*/

    let mut zip_writer = ZipWriter::new(out);
    let mut buffer = Vec::new();

    for mut fp in files {
        fp.file.read_to_end(&mut buffer).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Could not read from file, {:?}", e))
        })?;

        zip_writer
            .start_file(Path::new(&fp.path).to_string_lossy(), options)
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Could not add file path to ZIP, {:?}",
                    e
                ))
            })?;
        zip_writer.write(buffer.as_ref()).map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Could not write file to ZIP, {:?}",
                e
            ))
        })?;
        buffer.clear();

        /*} else if entry_metadata.is_dir() {
            let relative_path = zip_directory.join(current_entry_name).into_os_string();
            zip_writer
                .add_directory(relative_path.to_string_lossy(), options)
                .map_err(|_| {
                    ContextualError::ArchiveCreationDetailError(
                        "Could not add directory path to ZIP".to_string(),
                    )
                })?;
            paths_queue.push(entry_path.clone());
        }*/
    }

    zip_writer.finish().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Could not finish writing ZIP archive, {:?}",
            e
        ))
    })?;
    Ok(())
}
