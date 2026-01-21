use anyhow::{anyhow, Result};
use log::{error, info};
use std::path::{Path, PathBuf};
use tiny_http::{Header, Method, Response, Server, StatusCode};

pub fn run(port: u16, path: PathBuf) -> Result<()> {
    let root = resolve_root(path)?;

    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).map_err(|e| anyhow!("Failed to bind {}: {}", addr, e))?;

    info!("HTTP server listening on http://{}", addr);
    info!("Serving directory: {}", root.display());

    for request in server.incoming_requests() {
        if let Err(err) = handle_request(request, &root) {
            error!("Request handling error: {}", err);
        }
    }

    Ok(())
}

fn resolve_root(path: PathBuf) -> Result<PathBuf> {
    let root = if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    };

    if !root.exists() {
        return Err(anyhow!("Path does not exist: {}", root.display()));
    }

    let canonical = root.canonicalize().unwrap_or(root);
    Ok(canonical)
}

fn handle_request(request: tiny_http::Request, root: &Path) -> Result<()> {
    if request.method() != &Method::Get {
        let response = Response::empty(StatusCode(405));
        request.respond(response)?;
        return Ok(());
    }

    let url_path = request.url();
    let target_path = match resolve_target_path(root, url_path) {
        Some(path) => path,
        None => {
            let response = Response::empty(StatusCode(404));
            request.respond(response)?;
            return Ok(());
        }
    };

    if !target_path.exists() {
        let response = Response::empty(StatusCode(404));
        request.respond(response)?;
        return Ok(());
    }

    if target_path.is_dir() {
        let listing = build_directory_listing(root, &target_path, url_path)?;
        let mut response = Response::from_string(listing);
        let header = Header::from_bytes("Content-Type", "text/html; charset=utf-8")
            .map_err(|_| anyhow!("Invalid Content-Type header value"))?;
        response.add_header(header);
        request.respond(response)?;
        return Ok(());
    }

    let file = std::fs::File::open(&target_path)?;
    let mut response = Response::from_file(file);

    if let Some(mime) = mime_guess::from_path(&target_path).first() {
        let header = Header::from_bytes("Content-Type", mime.as_ref())
            .map_err(|_| anyhow!("Invalid Content-Type header value"))?;
        response.add_header(header);
    }

    request.respond(response)?;
    Ok(())
}

fn resolve_target_path(root: &Path, url: &str) -> Option<PathBuf> {
    let path_part = url.split('?').next().unwrap_or("");
    let trimmed = path_part.trim_start_matches('/');
    let decoded = urlencoding::decode(trimmed).ok()?.into_owned();

    let joined = if decoded.is_empty() {
        root.to_path_buf()
    } else {
        root.join(decoded.as_str())
    };

    let canonical = joined.canonicalize().ok()?;
    if !canonical.starts_with(root) {
        return None;
    }

    if canonical.is_dir() {
        let index = canonical.join("index.html");
        if index.exists() {
            return Some(index);
        }
    }

    Some(canonical)
}

fn build_directory_listing(root: &Path, dir: &Path, url: &str) -> Result<String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .collect();

    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_lowercase());

    let mut base_path = url.split('?').next().unwrap_or("").to_string();
    if !base_path.ends_with('/') {
        base_path.push('/');
    }

    let rel_dir = dir.strip_prefix(root).unwrap_or(dir);
    let title = if rel_dir.as_os_str().is_empty() {
        "/".to_string()
    } else {
        format!("/{}", rel_dir.to_string_lossy())
    };

    let mut body = String::new();
    body.push_str("<!doctype html><html><head><meta charset=\"utf-8\">");
    body.push_str(&format!("<title>Index of {}</title>", html_escape(&title)));
    body.push_str("</head><body>");
    body.push_str(&format!("<h1>Index of {}</h1><hr><ul>", html_escape(&title)));

    if !rel_dir.as_os_str().is_empty() {
        body.push_str("<li><a href=\"../\">../</a></li>");
    }

    for entry in entries {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        let mut display = name.to_string();
        let entry_path = entry.path();
        let is_dir = entry_path.is_dir();
        if is_dir {
            display.push('/');
        }

        let encoded_name = urlencoding::encode(&name);
        let href = if is_dir {
            format!("{}{}/", base_path, encoded_name)
        } else {
            format!("{}{}", base_path, encoded_name)
        };

        body.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            html_escape(&href),
            html_escape(&display)
        ));
    }

    body.push_str("</ul><hr></body></html>");
    Ok(body)
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
