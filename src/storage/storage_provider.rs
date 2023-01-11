use crate::archive::{ArchiveMethod, FileWithPath};
use crate::database::entities::file::File as DBFile;
use crate::settings::Settings;
use actix_files::NamedFile;
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use actix_web::web;
use futures::channel::mpsc::Receiver;
use futures::Stream;
use futures_util::future::FlattenStream;
use futures_util::FutureExt;
use mime::Mime;
use once_cell::sync::OnceCell;
use std::fs::File;
use std::io::{Read, Result as IoResult, Write};
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use std::{fs, io};
use tokio::io::BufWriter;
use tracing::error;

static UPLOAD_PATH: OnceCell<String> = OnceCell::new();

pub struct StorageProvider {}

impl StorageProvider {
    pub fn init(settings: &Settings) -> IoResult<()> {
        fs::create_dir_all(&settings.upload_path)?;

        UPLOAD_PATH.set(settings.upload_path.clone()).unwrap();
        Ok(())
    }
    pub fn get_direct_file_path(uuid: String) -> String {
        format!("{}/{}", UPLOAD_PATH.get().unwrap(), uuid)
    }
    pub async fn create_file_handle(uuid: String) -> actix_web::Result<File> {
        //decide if a new file is required of if the data will be appended to an existing partly uploaded file
        //vfile.create_or_get_id_from_existing_dir().await?;

        // File::create is blocking operation, use threadpool
        web::block(move || File::create(StorageProvider::get_direct_file_path(uuid)))
            .await?
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))
    }
    pub fn delete_file(uuid: String) -> std::io::Result<()> {
        fs::remove_file(StorageProvider::get_direct_file_path(uuid))?;
        Ok(())
    }
    pub fn get_named_file(file: &DBFile) -> actix_web::Result<NamedFile> {
        let mut named_file = NamedFile::open(Self::get_direct_file_path(file.uuid.to_string()))?;

        named_file = named_file.set_content_disposition(ContentDisposition {
            disposition: DispositionType::Inline,
            parameters: vec![DispositionParam::Filename(file.name.clone())],
        });
        if let Ok(mime) = Mime::from_str(file.mime.as_str()) {
            named_file = named_file.set_content_type(mime);
        }

        Ok(named_file)
    }
    pub fn get_compressed_file_stream(
        file: &DBFile,
        archive_method: ArchiveMethod,
    ) -> actix_web::Result<Receiver<io::Result<actix_web::web::Bytes>>> {
        // We will create the archive in a separate thread, and stream the content using a pipe.
        // The pipe is made of a futures channel, and an adapter to implement the `Write` trait.
        // Include 10 messages of buffer for erratic connection speeds.
        let (tx, rx) = futures::channel::mpsc::channel::<io::Result<actix_web::web::Bytes>>(10);
        let pipe = crate::pipe::Pipe::new(tx);

        let real_file = File::open(Self::get_direct_file_path(file.uuid.to_string()))?;

        let mut files: Vec<FileWithPath> = Vec::new();
        files.push(FileWithPath {
            file: real_file,
            path: (&file.name).clone(),
        });

        // Start the actual archive creation in a separate thread.
        std::thread::spawn(move || {
            if let Err(err) = archive_method.create_archive(files, pipe) {
                error!("Error during archive creation: {:?}", err);
            }
        });

        Ok(rx)
    }
}
