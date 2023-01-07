use crate::database::entities::file::File as DBFile;
use crate::settings::Settings;
use actix_files::NamedFile;
use actix_web::web;
use mime::Mime;
use once_cell::sync::OnceCell;
use std::fs;
use std::fs::File;
use std::io::Result as IoResult;
use std::str::FromStr;

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

        if let Ok(mime) = Mime::from_str(file.mime.as_str()) {
            named_file = named_file.set_content_type(mime);
        }

        Ok(named_file)
    }
}
