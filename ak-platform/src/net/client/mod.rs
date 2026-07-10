use eyre::Result;
use hyper_util::rt::TokioIo;
use interprocess::local_socket::{
    GenericFilePath,
    tokio::{Stream as LocalSocketStream, prelude::*},
};

use crate::string::PlatformString;

pub async fn connect(path: PlatformString) -> Result<TokioIo<LocalSocketStream>> {
    let name = path.for_current().to_fs_name::<GenericFilePath>()?;

    let stream = LocalSocketStream::connect(name).await?;
    Ok(TokioIo::new(stream))
}
