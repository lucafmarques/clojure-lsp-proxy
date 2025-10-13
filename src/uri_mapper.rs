use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
  time::SystemTime,
};

use serde_json::Value;
use sha1::{Digest, Sha1};
use url::Url;
use zip::ZipArchive;

use crate::error::{KotlinLSPWrapperError, KotlinLSPWrapperResult};

pub(crate) struct UriMapper {
  cache_root: PathBuf,
  jar_to_file: HashMap<String, Url>,
  file_to_jar: HashMap<String, Url>,
}

impl UriMapper {
  pub(crate) fn new() -> Self {
    let cache_root = dirs::cache_dir()
      .unwrap_or_else(|| PathBuf::from("."))
      .join("kotlin-lsp-proxy");
    fs::create_dir_all(&cache_root).ok();

    Self {
      cache_root,
      jar_to_file: HashMap::new(),
      file_to_jar: HashMap::new(),
    }
  }
  
  pub(crate) fn map_server_uri(&mut self, url: &Url) -> Url {
    if url.scheme() != "jar" {
      return url.clone();
    }

    // Serve from the memory cache if it is built.
    if let Some(cached_file) = self.jar_to_file.get(url.as_str()) {
      return cached_file.clone();
    }
    
    let stripped_jar = url.as_str().trim_start_matches("jar://");

    let Some((jar_path_raw, entry_path)) = stripped_jar.split_once("!")
    else {
      return url.clone();
    };

    let jar_path = if jar_path_raw.starts_with("/") {
      jar_path_raw.to_string()
    } else {
      match jar_path_raw.starts_with("localhost/") {
        true => format!("/{}", &jar_path_raw["localhost/".len()..]),
        false => format!("/{}", jar_path_raw),
      }
    };

    let file_path = match self.ensure_extracted(&jar_path, entry_path) {
      Ok(p) => p,
      Err(e) => {
        eprintln!("[proxy] Failed extracting {url}: {e}");
        return url.clone();
      }
    };

    let file_uri =
      Url::from_file_path(&file_path).unwrap_or_else(|_| url.clone());

    self.jar_to_file.insert(url.to_string(), file_uri.clone());
    self.file_to_jar.insert(file_uri.to_string(), url.clone());
    file_uri
  }

  pub(crate) fn ensure_extracted(
    &self,
    jar_abs_path: &str,
    entry_path: &str,
  ) -> KotlinLSPWrapperResult<PathBuf> {
    let meta = fs::metadata(jar_abs_path)?;
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let size = meta.len();

    let mut hasher = Sha1::new();
    hasher.update(jar_abs_path.as_bytes());
    hasher.update(size.to_le_bytes());
    if let Ok(dur) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
      hasher.update(dur.as_secs().to_le_bytes());
      hasher.update(dur.subsec_nanos().to_le_bytes());
    }

    let key = hex::encode(hasher.finalize());

    let jar_base = Path::new(jar_abs_path)
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_else(|| "unknown.jar".into());

    let dest = self
      .cache_root
      .join(key)
      .join(jar_base)
      .join(entry_path.trim_start_matches('/'));

    if dest.exists() {
      return Ok(dest);
    }

    if let Some(parent) = dest.parent() {
      fs::create_dir_all(parent)?;
    }

    let jar_file = fs::File::open(jar_abs_path)?;
    let mut zip = ZipArchive::new(jar_file)?;
    let norm_entry = entry_path.trim_start_matches('/');

    // Avoid path traversal.
    let norm_entry =
      Path::new(norm_entry)
        .components()
        .fold(PathBuf::new(), |mut acc, c| {
          use std::path::Component::*;
          match c {
            CurDir => {}
            ParentDir => {}
            RootDir => {}
            Normal(os_str) => acc.push(os_str),
            Prefix(_) => {}
          }
          acc
        });

    let entry_str = norm_entry.to_string_lossy().to_string();

    let mut found = None;
    for i in 0..zip.len() {
      let name = zip.by_index(i)?.name().to_string();
      if name == entry_str {
        found = Some(i);
        break;
      }
    }

    let idx = found.ok_or_else(|| {
      KotlinLSPWrapperError::General(format!(
        "Entry not found in jar: {}",
        entry_str
      ))
    })?;

    let mut zf = zip.by_index(idx)?;
    let mut out = fs::File::create(&dest)?;
    std::io::copy(&mut zf, &mut out)?;

    #[cfg(unix)]
    {
      let mut perm = out.metadata()?.permissions();
      perm.set_readonly(true);
      fs::set_permissions(&dest, perm)?;
    }

    Ok(dest)
  }

  pub(crate) fn remap_uris_in_value(
    &mut self,
    v: &mut Value,
    server_to_client: bool,
  ) {
    match v {
      Value::Object(obj) => {
        for (k, val) in obj.iter_mut().collect::<Vec<_>>() {
          if matches!(&*val, Value::String(_)) {
            let key_is_uri_like =
              *k == "uri" || *k == "targetUri" || *k == "target";

            if
              key_is_uri_like &&
              let Value::String(converted_string) = val &&
              let Ok(u) = Url::parse(converted_string) &&
              server_to_client {
                let mapped = self.map_server_uri(&u);
                if mapped.as_str() != converted_string {
                  *converted_string = mapped.to_string();
                }
              }
          }
        }

        for val in obj.values_mut() {
          self.remap_uris_in_value(val, server_to_client);
        }
      }
      Value::Array(arr) => {
        for elem in arr {
          self.remap_uris_in_value( elem, server_to_client);
        }
      },
      _ => {}
    }
  }
}

