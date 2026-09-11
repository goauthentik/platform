use ak_platform::generated::sys_directory::{
    GetRequest, Group as AKGroup, User, system_directory_client::SystemDirectoryClient,
};
use ak_platform::grpc::{Code, GrpcError, GrpcResult, grpc_request};
use libnss::interop::Response;

pub trait DirectoryBridge {
    fn list_users(&self) -> GrpcResult<Vec<User>>;
    fn get_user(&self, req: GetRequest) -> GrpcResult<User>;
    fn list_groups(&self) -> GrpcResult<Vec<AKGroup>>;
    fn get_group(&self, req: GetRequest) -> GrpcResult<AKGroup>;
}

pub struct GrpcDirectoryBridge;

impl DirectoryBridge for GrpcDirectoryBridge {
    fn list_users(&self) -> GrpcResult<Vec<User>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_users(()).await?))
            .map(|r| r.into_inner().users)
    }

    fn get_user(&self, req: GetRequest) -> GrpcResult<User> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_user(req).await?) }
        })
        .map(|r| r.into_inner())
    }

    fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_groups(()).await?))
            .map(|r| r.into_inner().groups)
    }

    fn get_group(&self, req: GetRequest) -> GrpcResult<AKGroup> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_group(req).await?) }
        })
        .map(|r| r.into_inner())
    }
}

pub trait ErrMap<T> {
    fn to_response<C: ToString>(self, context: C) -> Response<T>;
}

impl<T> ErrMap<T> for GrpcResult<T> {
    fn to_response<C: ToString>(self, context: C) -> Response<T> {
        match self {
            Ok(t) => Response::Success(t),
            Err(e) => match e {
                GrpcError::Status(s) => match s.code() {
                    Code::NotFound => Response::NotFound,
                    other => {
                        tracing::warn!(
                            "{}: {} ({})",
                            context.to_string(),
                            other,
                            other.description()
                        );
                        Response::Unavail
                    }
                },
                other => {
                    tracing::warn!("{}: {}", context.to_string(), other);
                    Response::Unavail
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::grpc::Status;

    /// The whole point of the mapping: sysd's `Status::not_found` must reach NSS
    /// as NOTFOUND. It used to arrive as UNAVAIL, i.e. "this backend is broken".
    #[test]
    fn not_found_is_the_only_miss() {
        let res: GrpcResult<u8> = Err(Status::not_found("user not found").into());
        assert!(matches!(res.to_response("ctx"), Response::NotFound));
    }

    #[test]
    fn every_other_code_is_unavail() {
        for code in [
            Code::Ok,
            Code::Cancelled,
            Code::Unknown,
            Code::InvalidArgument,
            Code::DeadlineExceeded,
            Code::AlreadyExists,
            Code::PermissionDenied,
            Code::ResourceExhausted,
            Code::FailedPrecondition,
            Code::Aborted,
            Code::OutOfRange,
            Code::Unimplemented,
            Code::Internal,
            Code::Unavailable,
            Code::DataLoss,
            Code::Unauthenticated,
        ] {
            let res: GrpcResult<u8> = Err(Status::new(code, "x").into());
            assert!(
                matches!(res.to_response("ctx"), Response::Unavail),
                "{code:?} must not be reported as a miss"
            );
        }
    }

    /// Failures that never reached sysd carry no gRPC code at all, and are
    /// likewise not misses.
    #[test]
    fn non_status_errors_are_unavail() {
        let res: GrpcResult<u8> = Err(GrpcError::Other(eyre::eyre!("no tokio runtime")));
        assert!(matches!(res.to_response("ctx"), Response::Unavail));
    }

    #[test]
    fn success_passes_the_value_through() {
        let res: GrpcResult<u8> = Ok(7);
        assert!(matches!(res.to_response("ctx"), Response::Success(7)));
    }

    /// TryAgain risks an unbounded glibc retry loop and Return would abort the
    /// nsswitch chain, so neither may ever be produced. See `to_response`'s docs.
    #[test]
    fn never_try_again_or_return() {
        for err in [
            GrpcError::Status(Status::not_found("x")),
            GrpcError::Status(Status::internal("x")),
            GrpcError::Other(eyre::eyre!("x")),
        ] {
            let res: Response<u8> = Err(err).to_response("ctx");
            assert!(!matches!(res, Response::TryAgain | Response::Return));
        }
    }
}
