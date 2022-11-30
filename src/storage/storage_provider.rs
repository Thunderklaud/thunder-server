use crate::settings::Settings;
use actix_web::web;
use once_cell::sync::OnceCell;
use std::fs;
use std::fs::File;
use std::io::Result as IoResult;

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
}
