use std::error::Error;

use hyper_util::rt::TokioIo;
use interprocess::local_socket::{
    tokio::{prelude::*, Stream as LocalSocketStream},
    GenericFilePath,
};

use crate::platform::string::PlatformString;

pub async fn connect(
    path: PlatformString,
) -> Result<TokioIo<LocalSocketStream>, Box<dyn Error + Send + Sync>> {
    let name = path.for_current().to_fs_name::<GenericFilePath>()?;

    let stream = LocalSocketStream::connect(name).await?;
    Ok(TokioIo::new(stream))
}
