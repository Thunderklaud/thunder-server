use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;

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

    /// Make an archive out of the given FileWithPath vec, and write the output to the given writer.
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

fn create_zip_from_file_with_path_vec<W>(out: W, files: Vec<FileWithPath>) -> actix_web::Result<()>
where
    W: std::io::Write + std::io::Seek,
{
    let options = write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
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
    }

    zip_writer.finish().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Could not finish writing ZIP archive, {:?}",
            e
        ))
    })?;
    Ok(())
}
