use std::path::PathBuf;

use crate::request::Request;
use color_eyre::{Result, eyre::eyre, Report};

#[derive(Clone, Debug)]
pub struct InitRequest {
    pub name: PathBuf,
}

impl TryFrom<InitRequest> for Request {
    type Error = Report;

    fn try_from(request: InitRequest) -> Result<Request> {
        if request.name.extension().is_some() {
            return Err(eyre!("Path is not a directory: {}", request.name.display()));
        } else if request.name.exists() {
            return Err(eyre!("Directory already exists: {}", request.name.display()));
        }

        Ok(Request::Init(request))
    }
}
