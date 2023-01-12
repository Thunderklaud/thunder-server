use actix_web::HttpResponse;
use futures::channel::mpsc::Receiver;
use std::io;
use std::str::FromStr;

use crate::archive::ArchiveMethod;
use mongodb::bson::oid::ObjectId;

pub fn extract_object_id(
    id: Option<&String>,
    default_if_none: ObjectId,
) -> actix_web::Result<ObjectId> {
    Ok(match id {
        Some(parent_id) if !parent_id.is_empty() => {
            ObjectId::from_str(parent_id).map_err(|e| actix_web::error::ErrorBadRequest(e))?
        }
        _ => default_if_none,
    })
}

pub fn extract_object_id_or_die(id: Option<&String>) -> actix_web::Result<ObjectId> {
    if let Some(id) = id {
        return Ok(ObjectId::from_str(id).map_err(|e| actix_web::error::ErrorBadRequest(e))?);
    }

    Err(actix_web::error::ErrorBadRequest("no ObjectId to extract"))
}

pub fn get_archive_file_stream_http_response(
    archive_method: ArchiveMethod,
    file_name: String,
    rx: Receiver<io::Result<actix_web::web::Bytes>>,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type(archive_method.content_type())
        .append_header(archive_method.content_encoding())
        .append_header(("Content-Transfer-Encoding", "binary"))
        .append_header((
            "Content-Disposition",
            format!("attachment; filename={:?}", file_name),
        ))
        .body(actix_web::body::BodyStream::new(rx)))
}
