mod error;
mod transport;
mod uri_mapper;

use serde_json::Value;
use tokio::process::Command;

use crate::{
  error::KotlinLSPWrapperResult,
  transport::{read_message, write_message},
  uri_mapper::UriMapper,
};

#[tokio::main]
async fn main() -> KotlinLSPWrapperResult<()> {
  let result = ensure_lsp_is_installed()
    .await
    .expect("Couldn't execute command which. Terminating.");

  if !result {
    eprintln!("Couldn't find kotlin-lsp. Are you sure it's installed?");
    std::process::exit(0);
  }

  let mut child = Command::new("kotlin-lsp")
    .arg("--stdio")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .expect("Failed to kotlin-lsp command.");

  let mut child_stdin = child.stdin.take().unwrap();
  let mut child_stdout = child.stdout.take().unwrap();

  let mut editor_r = tokio::io::stdin();
  let mut editor_w = tokio::io::stdout();

  let mut mapper = UriMapper::new();

  loop {
    tokio::select! {
      // From editor -> Server
      msg = read_message(&mut editor_r) => {
        let Some(body) = msg? else {break;};

        // We just forward messages from Helix to the LSP.
        write_message(&mut child_stdin, &body).await?;
      }

      // from server -> editor
      msg = read_message(&mut child_stdout) => {
        let Some(body) = msg? else { break; };
        let mut v: Value = match serde_json::from_slice(&body) {
          Ok(v) => v,
          Err(e) => {
            eprintln!("[Proxy] JSON Parse error from server: {e}");
            write_message(&mut editor_w, &body).await?;
            continue;
          }
        };

        if let Some(result) = v.get_mut("result") {
          mapper.remap_uris_in_value(result, true);
        }

        if let Some(params) = v.get_mut("params") {
          mapper.remap_uris_in_value(params, true);
        }

        let out = serde_json::to_vec(&v)?;
        write_message(&mut editor_w, &out).await?;
      }
    }
  }
  Ok(())
}

// A really dumb-function to check whether kotlin-lsp is installed.
async fn ensure_lsp_is_installed() -> KotlinLSPWrapperResult<bool> {
  let check_installed =
    Command::new("which").arg("kotlin-lsp").output().await?;

  Ok(check_installed.status.success())
}
