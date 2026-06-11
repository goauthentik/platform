use std::error::Error;

use hyper_util::rt::TokioIo;
use interprocess::local_socket::{
    tokio::{prelude::*, Stream as LocalSocketStream},
    GenericFilePath,
};
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;

use crate::platform::string::PlatformString;

pub async fn connect(
    path: PlatformString,
) -> Result<TokioIo<LocalSocketStream>, Box<dyn Error + Send + Sync>> {
    #[cfg(unix)]
    let name = path.for_current().to_fs_name::<GenericFilePath>()?;
    #[cfg(windows)]
    let name = path.for_current().to_ns_name::<GenericNamespaced>()?;

    let stream = LocalSocketStream::connect(name).await?;
    Ok(TokioIo::new(stream))
}
