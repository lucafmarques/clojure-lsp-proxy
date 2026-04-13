mod error;
mod transport;
mod uri_mapper;

use serde_json::Value;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::{
  error::ClojureLspProxyResult,
  transport::{Message, read_message, write_message, write_raw},
  uri_mapper::UriMapper,
};

#[tokio::main]
async fn main() -> ClojureLspProxyResult<()> {
  let result = ensure_lsp_is_installed()
    .await
    .expect("Couldn't execute command which. Terminating.");

  if !result {
    eprintln!("Couldn't find clojure-lsp. Are you sure it's installed?");
    std::process::exit(0);
  }

  let mut child = Command::new("clojure-lsp")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .expect("Failed to start clojure-lsp command.");

  let mut child_stdin = child.stdin.take().unwrap();
  let child_stdout = child.stdout.take().unwrap();

  let editor_r = tokio::io::stdin();
  let mut editor_w = tokio::io::stdout();

  let mut mapper = UriMapper::new();

  // Dedicated reader tasks avoid tokio::select! cancelling
  // read_message mid-read and losing partially-read bytes.
  let (editor_tx, mut editor_rx) = mpsc::channel::<Message>(64);
  let (server_tx, mut server_rx) = mpsc::channel::<Message>(64);

  tokio::spawn(async move {
    let mut r = editor_r;
    loop {
      match read_message(&mut r).await {
        Ok(Some(msg)) => {
          if editor_tx.send(msg).await.is_err() {
            break;
          }
        }
        Ok(None) => break,
        Err(e) => {
          eprintln!("[proxy] Editor read error: {e}");
          break;
        }
      }
    }
  });

  tokio::spawn(async move {
    let mut r = child_stdout;
    loop {
      match read_message(&mut r).await {
        Ok(Some(msg)) => {
          if server_tx.send(msg).await.is_err() {
            break;
          }
        }
        Ok(None) => break,
        Err(e) => {
          eprintln!("[proxy] Server read error: {e}");
          break;
        }
      }
    }
  });

  loop {
    tokio::select! {
      msg = editor_rx.recv() => {
        let Some(msg) = msg else { break; };
        match msg {
          Message::Lsp(body) => write_message(&mut child_stdin, &body).await?,
          Message::Raw(data) => write_raw(&mut child_stdin, &data).await?,
        }
      }

      msg = server_rx.recv() => {
        let Some(msg) = msg else { break; };
        match msg {
          Message::Raw(data) => {
            write_raw(&mut editor_w, &data).await?;
          }
          Message::Lsp(body) => {
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
    }
  }
  Ok(())
}

// A really dumb-function to check whether clojure-lsp is installed.
async fn ensure_lsp_is_installed() -> ClojureLspProxyResult<bool> {
  let check_installed =
    Command::new("which").arg("clojure-lsp").output().await?;

  Ok(check_installed.status.success())
}
