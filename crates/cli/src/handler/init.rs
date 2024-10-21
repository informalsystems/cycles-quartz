use async_trait::async_trait;
use cargo_generate::{generate, GenerateArgs, TemplatePath, Vcs};
use color_eyre::{eyre::Context, owo_colors::OwoColorize, Report, Result};
use tokio::fs;
use tracing::info;

use crate::{
    config::Config,
    handler::Handler,
    request::init::InitRequest,
    response::{init::InitResponse, Response},
};

#[async_trait]
impl Handler for InitRequest {
    type Response = Response;

    // TODO: Add non-template init method
    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report> {
        let config = config.as_ref();
        info!("{}", "\nPeforming Init".blue().bold());

        let parent = self
            .name
            .parent()
            .map(|p| p.to_path_buf())
            .expect("path already validated");
        fs::create_dir_all(&parent)
            .await
            .wrap_err("Error creating directories to target app folder")?;

        let file_name = self
            .name
            .file_name()
            .and_then(|f| f.to_str())
            .expect("path already validated");

        let wasm_pack_args = GenerateArgs {
            name: Some(file_name.to_string()),
            destination: Some(config.app_dir.join(parent)),
            overwrite: true,
            vcs: Some(Vcs::Git),
            template_path: TemplatePath {
                git: Some("https://github.com/informalsystems/cycles-quartz.git".to_string()),
                ..TemplatePath::default()
            },
            ..GenerateArgs::default()
        };

        let result_dir = generate(wasm_pack_args)
            .expect("something went wrong!")
            .display()
            .to_string();

        info!("\n{}", "It's TEE time.".green().bold());
        Ok(InitResponse { result_dir }.into())
    }
}
