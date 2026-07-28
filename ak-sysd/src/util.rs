use tonic::Status;

pub fn to_status<E: std::fmt::Display>(e: E) -> Status {
    Status::internal(e.to_string())
}
