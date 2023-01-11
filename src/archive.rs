use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use actix_web::http::header::ContentEncoding;
use libflate::gzip::Encoder;
use serde::Deserialize;
use strum::{Display, EnumIter, EnumString};
use tar::Builder;

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
            //ArchiveMethod::Zip => zip_dir(files, out),
            ArchiveMethod::Zip => unimplemented!(),
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
