use eyre::Report;
use libnss::interop::Response;

/// Map a gRPC/bridge error to the correct NSS response.
///
/// A gRPC `NotFound` status becomes `Response::NotFound` (`NSS_STATUS_NOTFOUND`);
/// any other error becomes `Response::Unavail` (`NSS_STATUS_UNAVAIL`).
pub fn response_for_error<T>(context: &str, err: &Report) -> Response<T> {
    if err
        .downcast_ref::<tonic::Status>()
        .is_some_and(|s| s.code() == tonic::Code::NotFound)
    {
        Response::NotFound
    } else {
        tracing::warn!("{context}: {err:?}");
        Response::Unavail
    }
}
