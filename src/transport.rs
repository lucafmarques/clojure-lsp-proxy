use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{KotlinLSPWrapperError, KotlinLSPWrapperResult};

// Minimal framing for JSON-RPC: read/write with Content-Length headers.
pub(crate) async fn read_message<R: AsyncReadExt + Unpin>(
  r: &mut R,
) -> KotlinLSPWrapperResult<Option<Vec<u8>>> {
  let mut content_length: Option<usize> = None;
  let mut line = Vec::new();
  #[allow(unused_assignments)]
  let mut header_ended = false;

  loop {
    line.clear();

    loop {
      let mut byte = [0u8; 1];
      let n = r.read(&mut byte).await?;
      if n == 0 {
        // EOF: if we haven't read any headers, signal end.
        if content_length.is_none() {
          return Ok(None);
        } else {
          return Err(KotlinLSPWrapperError::General("EOF mid-headers".into()));
        }
      }

      line.push(byte[0]);
      if line.ends_with(b"\r\n") {
        break;
      }
    }

    let line_str = String::from_utf8_lossy(&line);
    let trimmed = line_str.trim_end_matches("\r\n");
    if trimmed.is_empty() {
      header_ended = true;
      break;
    }

    if let Some(v) = trimmed.strip_prefix("Content-Length:") {
      content_length = Some(v.trim().parse()?);
    }
    // ignore other headers.
  }

  if !header_ended {
    return Ok(None);
  }

  let len = content_length.ok_or_else(|| {
    KotlinLSPWrapperError::General("Missing Content-Length".into())
  })?;

  let mut body = vec![0u8; len];
  let mut read = 0usize;

  while read < len {
    let n = r.read(&mut body[read..]).await?;

    if n == 0 {
      return Err(KotlinLSPWrapperError::General(
        "EOF while reading body".into(),
      ));
    }
    read += n;
  }

  Ok(Some(body))
}

pub(crate) async fn write_message<W: AsyncWriteExt + Unpin>(
  w: &mut W,
  body: &[u8],
) -> KotlinLSPWrapperResult<()> {
  w.write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
    .await?;
  w.write_all(body).await?;
  w.flush().await?;
  Ok(())
}
