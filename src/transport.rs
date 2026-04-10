use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{ClojureLspProxyError, ClojureLspProxyResult};

pub(crate) enum Message {
  /// A properly framed LSP message (body only, without headers).
  Lsp(Vec<u8>),
  /// Raw bytes that were not properly framed — forward as-is.
  Raw(Vec<u8>),
}

// Minimal framing for JSON-RPC: read/write with Content-Length headers.
// When data arrives without Content-Length, returns it as Raw for passthrough.
pub(crate) async fn read_message<R: AsyncReadExt + Unpin>(
  r: &mut R,
) -> ClojureLspProxyResult<Option<Message>> {
  let mut content_length: Option<usize> = None;
  let mut raw = Vec::new();
  let mut line = Vec::new();
  #[allow(unused_assignments)]
  let mut header_ended = false;

  loop {
    line.clear();

    loop {
      let mut byte = [0u8; 1];
      let n = r.read(&mut byte).await?;
      if n == 0 {
        if content_length.is_none() && raw.is_empty() {
          return Ok(None);
        } else if !raw.is_empty() {
          return Ok(Some(Message::Raw(raw)));
        } else {
          return Err(ClojureLspProxyError::General("EOF mid-headers".into()));
        }
      }

      line.push(byte[0]);
      raw.push(byte[0]);
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

  let Some(len) = content_length else {
    return Ok(Some(Message::Raw(raw)));
  };

  let mut body = vec![0u8; len];
  let mut read = 0usize;

  while read < len {
    let n = r.read(&mut body[read..]).await?;

    if n == 0 {
      return Err(ClojureLspProxyError::General(
        "EOF while reading body".into(),
      ));
    }
    read += n;
  }

  Ok(Some(Message::Lsp(body)))
}

pub(crate) async fn write_message<W: AsyncWriteExt + Unpin>(
  w: &mut W,
  body: &[u8],
) -> ClojureLspProxyResult<()> {
  w.write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
    .await?;
  w.write_all(body).await?;
  w.flush().await?;
  Ok(())
}

pub(crate) async fn write_raw<W: AsyncWriteExt + Unpin>(
  w: &mut W,
  data: &[u8],
) -> ClojureLspProxyResult<()> {
  w.write_all(data).await?;
  w.flush().await?;
  Ok(())
}
