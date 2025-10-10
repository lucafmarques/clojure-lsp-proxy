// We should:
// Check whether the kotlin-lsp is installed. If it is not, we need to
// gracefully fail.
// For now, it should be a requirement for the user to have it installed
// themselves and then we can download it for them later.
//
//
//
//

mod error;

use tokio::{
  io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
  process::Command,
  try_join,
};

use crate::error::KotlinLSPWrapperResult;

#[tokio::main]
async fn main() {
  let result = ensure_lsp_is_installed()
    .await
    .expect("Couldn't execute command which. Terminating.");

  if !result {
    eprintln!("Couldn't find kotlin-lsp. Are you sure it's installed?");
    std::process::exit(0);
  }

  // eprintln!("WAPING");
  let child = Command::new("kotlin-lsp")
    .arg("--stdio")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .expect("Failed to execute command.");

  let mut server_stdin = child.stdin.unwrap();
  let server_stdout = child.stdout.unwrap();

  let stdin = io::stdin();

  let from_lsp = async {
    let mut reader = BufReader::new(server_stdout);
    let mut stdout = io::stdout();

    io::copy(&mut reader, &mut stdout).await
  };

  let to_lsp = async {
    let mut stdin = BufReader::new(stdin);

    loop {
      let mut buffer = vec![0; 6000];
      let bytes_read = stdin
        .read(&mut buffer)
        .await
        .expect("Unable to read incoming client notification");

      if bytes_read == 0 {
        break; // EOF reached
      }

      server_stdin
        .write_all(&buffer[..bytes_read])
        .await
        .expect("Unable to forward client notification to server");
    }
    io::copy(&mut stdin, &mut server_stdin).await
  };

  _ = try_join!(to_lsp, from_lsp);

  // let mut server_stdin = command.stdin.take().unwrap();
  // let mut server_stdout = command.stdout.take().unwrap();

  // let mut editor_stdin = tokio::io::stdin();
  // let mut editor_stdout = tokio::io::stdout();

  // loop {
  // tokio::select! {
  //   msg =
  // }
  // }
}

async fn ensure_lsp_is_installed() -> KotlinLSPWrapperResult<bool> {
  let check_installed =
    Command::new("which").arg("kotlin-lsp").output().await?;

  Ok(check_installed.status.success())
}
