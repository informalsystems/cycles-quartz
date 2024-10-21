use crate::request::Request;

#[derive(Clone, Debug)]
pub struct PrintFmspcRequest;

impl From<PrintFmspcRequest> for Request {
    fn from(request: PrintFmspcRequest) -> Self {
        Self::PrintFmspc(request)
    }
}
