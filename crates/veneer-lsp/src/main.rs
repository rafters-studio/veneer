//! Language server for `.tsx` and `.astro` files. It speaks LSP on stdio and holds no check logic:
//! one Node worker (`packages/veneer/src/worker.ts`) answers over JSON lines, so one check gives
//! one answer. It never fails the editor: without a worker it warns once and keeps answering.

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Notification as _, PublishDiagnostics, ShowMessage,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    MessageType, Uri,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;

enum Event {
    Lsp(Message),
    LspClosed,
    Worker(Value),
    WorkerEnded(io::Result<ExitStatus>),
}

/// The path of a `file:` URI, percent-decoded.
fn decoded_path(uri: &str) -> Option<String> {
    let uri: Uri = uri.parse().ok()?;
    let path = uri.path().as_estr().decode().into_string().ok()?;
    Some(path.into_owned())
}

/// The inverse of `decoded_path`, for a file the client never named.
fn encoded_path(file: &str) -> String {
    file.bytes().fold(String::new(), |mut encoded, byte| {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
        encoded
    })
}

/// The project-relative file of a URI, when the worker is to be sent it.
fn file_of(root: &str, uri: &str) -> Option<String> {
    let path = decoded_path(uri)?;
    let file = path.strip_prefix(root)?.strip_prefix('/')?;
    let taken = file.ends_with(".tsx") || file.ends_with(".astro");
    let inside = !file.split('/').any(|part| part == "..");
    (taken && inside).then(|| file.to_string())
}

fn position(line: &Value, column: &Value) -> Value {
    let zero_based = |value: &Value| value.as_u64().unwrap_or(1).saturating_sub(1);
    json!({"line": zero_based(line), "character": zero_based(column)})
}

/// A worker's `LintDiagnostic` as the LSP publishes it.
fn lsp_diagnostic(diagnostic: &Value) -> Value {
    let severity = match diagnostic["severity"].as_str() {
        Some("warning") => 2,
        Some("info") => 3,
        Some("hint") => 4,
        _ => 1,
    };
    // A language server's message is a string or markup content.
    let message = &diagnostic["message"];
    let message = message
        .as_str()
        .or_else(|| message["value"].as_str())
        .unwrap_or("");
    let help = diagnostic["help"]
        .as_str()
        .map(|help| format!("\nhelp: {help}"))
        .unwrap_or_default();
    json!({
        "range": {
            "start": position(&diagnostic["line"], &diagnostic["column"]),
            "end": position(&diagnostic["endLine"], &diagnostic["endColumn"]),
        },
        "severity": severity,
        "code": diagnostic["code"],
        "source": "veneer",
        "message": format!("{message}{help}"),
    })
}

fn list_of(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

struct Server {
    connection: Connection,
    root: String,
    root_uri: String,
    /// `None` once the worker cannot start or has ended: requests are then answered and nothing is
    /// sent. Dropping it ends the worker.
    worker: Option<ChildStdin>,
    ready: bool,
    in_flight: bool,
    /// Lines for the worker in order. At most one is a text: the newest.
    waiting: VecDeque<Value>,
    next_seq: i64,
    /// Per file, the seq of its newest text or close. An older result is not published.
    latest: HashMap<String, i64>,
    /// The URI the client named each file by.
    uris: HashMap<String, String>,
    /// The files whose last publish was not empty.
    showing: BTreeSet<String>,
}

impl Server {
    fn notify(&self, method: &str, params: Value) {
        let notification = Notification::new(method.to_string(), params);
        // A client that has gone away ends the event loop by itself.
        let _ = self.connection.sender.send(notification.into());
    }

    fn warn(&self, reason: &str) {
        self.notify(
            ShowMessage::METHOD,
            json!({"type": MessageType::WARNING, "message": format!("veneer: {reason}")}),
        );
    }

    fn end_worker(&mut self, reason: &str) {
        if self.worker.take().is_some() {
            self.waiting.clear();
            self.warn(reason);
        }
    }

    fn enqueue(&mut self, line: Value) {
        if self.worker.is_some() {
            self.waiting.push_back(line);
            self.send_next();
        }
    }

    fn send_next(&mut self) {
        if !self.ready || self.in_flight {
            return;
        }
        let Some(line) = self.waiting.pop_front() else {
            return;
        };
        let Some(worker) = self.worker.as_mut() else {
            return;
        };
        match writeln!(worker, "{line}").and_then(|()| worker.flush()) {
            Ok(()) => self.in_flight = true,
            Err(error) => self.end_worker(&format!("the worker stopped reading: {error}")),
        }
    }

    fn next_seq(&mut self) -> i64 {
        self.next_seq += 1;
        self.next_seq
    }

    fn on_text(&mut self, uri: &str, text: String) {
        let Some(file) = file_of(&self.root, uri) else {
            return;
        };
        let seq = self.next_seq();
        self.latest.insert(file.clone(), seq);
        self.uris.insert(file.clone(), uri.to_string());
        self.waiting.retain(|line| line.get("text").is_none());
        self.enqueue(json!({"seq": seq, "file": file, "text": text}));
    }

    fn on_close(&mut self, uri: &str) {
        let Some(file) = file_of(&self.root, uri) else {
            return;
        };
        let seq = self.next_seq();
        self.latest.insert(file.clone(), seq);
        self.enqueue(json!({"seq": seq, "close": file}));
    }

    fn on_notification(&mut self, notification: Notification) {
        let Notification { method, params } = notification;
        match method.as_str() {
            DidOpenTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidOpenTextDocumentParams>(params) {
                    let document = params.text_document;
                    self.on_text(document.uri.as_str(), document.text);
                }
            }
            DidChangeTextDocument::METHOD => {
                if let Ok(mut params) =
                    serde_json::from_value::<DidChangeTextDocumentParams>(params)
                {
                    if let Some(change) = params.content_changes.pop() {
                        self.on_text(params.text_document.uri.as_str(), change.text);
                    }
                }
            }
            DidCloseTextDocument::METHOD => {
                if let Ok(params) = serde_json::from_value::<DidCloseTextDocumentParams>(params) {
                    self.on_close(params.text_document.uri.as_str());
                }
            }
            DidSaveTextDocument::METHOD => {
                let seq = self.next_seq();
                self.enqueue(json!({"seq": seq, "save": true}));
            }
            _ => {}
        }
    }

    fn publish(&mut self, seq: i64, file: &str, diagnostics: &[Value]) {
        if self.latest.get(file).is_some_and(|newest| *newest > seq) {
            return;
        }
        if diagnostics.is_empty() {
            self.showing.remove(file);
        } else {
            self.showing.insert(file.to_string());
        }
        let uri = self.uris.get(file).cloned().unwrap_or_else(|| {
            let root = self.root_uri.trim_end_matches('/');
            format!("{root}/{}", encoded_path(file))
        });
        let diagnostics: Vec<Value> = diagnostics.iter().map(lsp_diagnostic).collect();
        self.notify(
            PublishDiagnostics::METHOD,
            json!({"uri": uri, "diagnostics": diagnostics}),
        );
    }

    fn publish_save(&mut self, seq: i64, files: &Map<String, Value>) {
        let cleared: Vec<String> = self
            .showing
            .iter()
            .filter(|file| !files.contains_key(*file))
            .cloned()
            .collect();
        for (file, diagnostics) in files {
            self.publish(seq, file, list_of(diagnostics));
        }
        for file in cleared {
            self.publish(seq, &file, &[]);
        }
    }

    fn on_worker(&mut self, line: &Value) {
        if let Some(ready) = line["ready"].as_bool() {
            if ready {
                self.ready = true;
            } else {
                let reason = line["message"].as_str().unwrap_or("the worker failed");
                self.end_worker(reason);
            }
        } else if let Some(seq) = line["seq"].as_i64() {
            self.in_flight = false;
            if let Some(files) = line["files"].as_object() {
                self.publish_save(seq, files);
            } else if let Some(file) = line["file"].as_str() {
                self.publish(seq, file, list_of(&line["diagnostics"]));
            }
        }
        self.send_next();
    }

    fn on_request(&self, request: Request) {
        let _ = self
            .connection
            .sender
            .send(Response::new_ok(request.id, Value::Null).into());
    }
}

/// Starts the worker, a thread forwarding its lines and then its end.
fn start_worker(
    worker: &str,
    root: &str,
    events: mpsc::Sender<Event>,
) -> io::Result<Option<ChildStdin>> {
    let mut child = Command::new("node")
        .args([worker, root])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    thread::spawn(move || {
        if let Some(stdout) = stdout {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str(&line) {
                    if events.send(Event::Worker(value)).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = events.send(Event::WorkerEnded(child.wait()));
    });
    Ok(stdin)
}

fn worker_option(args: impl Iterator<Item = String>) -> Option<String> {
    let mut args = args.skip_while(|arg| arg != "--worker");
    args.next()?;
    args.next()
}

fn main() -> Result<(), Box<dyn Error>> {
    let option = worker_option(std::env::args());
    let (connection, io_threads) = Connection::stdio();
    let params = connection.initialize(json!({
        "textDocumentSync": {"openClose": true, "change": 1, "save": true},
    }))?;

    let (events, inbox) = mpsc::channel();
    let forward = events.clone();
    let receiver = connection.receiver.clone();
    thread::spawn(move || {
        for message in receiver {
            if forward.send(Event::Lsp(message)).is_err() {
                return;
            }
        }
        let _ = forward.send(Event::LspClosed);
    });

    let root_uri = params["rootUri"].as_str().unwrap_or("").to_string();
    let mut server = Server {
        connection,
        root: decoded_path(&root_uri)
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string(),
        root_uri,
        worker: None,
        ready: false,
        in_flight: false,
        waiting: VecDeque::new(),
        next_seq: 0,
        latest: HashMap::new(),
        uris: HashMap::new(),
        showing: BTreeSet::new(),
    };
    // No default path: opening a folder must not run a worker that folder supplied.
    if let Some(worker) = option.filter(|_| !server.root.is_empty()) {
        match start_worker(&worker, &server.root, events) {
            Ok(stdin) => server.worker = stdin,
            Err(error) => server.warn(&format!("node could not start {worker}: {error}")),
        }
    } else {
        server.warn("the server needs --worker and a client rootUri, so there are no diagnostics");
    }

    for event in inbox {
        match event {
            Event::Lsp(Message::Request(request)) => {
                // Shutdown is answered like any request; the loop ends on the exit that follows.
                server.on_request(request);
            }
            Event::Lsp(Message::Notification(notification)) => {
                if notification.method == Exit::METHOD {
                    break;
                }
                server.on_notification(notification);
            }
            Event::Lsp(Message::Response(_)) => {}
            Event::LspClosed => break,
            Event::Worker(line) => server.on_worker(&line),
            Event::WorkerEnded(status) => {
                let ended = match status {
                    Ok(status) => format!("the worker ended ({status}); see the server log"),
                    Err(error) => format!("the worker ended: {error}"),
                };
                server.end_worker(&ended);
            }
        }
    }

    drop(server);
    io_threads.join()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_project_relative_and_decoded() {
        let uri = "file:///a%20b/src/pages/%5Bslug%5D.astro";
        assert_eq!(
            file_of("/a b", uri).as_deref(),
            Some("src/pages/[slug].astro")
        );
    }

    #[test]
    fn a_file_outside_the_root_or_of_another_kind_is_not_sent() {
        assert_eq!(file_of("/a", "file:///b/page.astro"), None);
        assert_eq!(file_of("/a", "file:///ab/page.astro"), None);
        assert_eq!(file_of("/a", "file:///a/../b/page.astro"), None);
        assert_eq!(file_of("/a", "file:///a/styles.css"), None);
        assert_eq!(file_of("/a", "file:///a/util.ts"), None);
        assert_eq!(
            file_of("/a", "file:///a/page.tsx").as_deref(),
            Some("page.tsx")
        );
    }

    #[test]
    fn an_encoded_path_decodes_to_itself() {
        let file = "src/pages/[slug] a.astro";
        assert_eq!(encoded_path(file), "src/pages/%5Bslug%5D%20a.astro");
        let uri = format!("file:///{}", encoded_path(file));
        assert_eq!(decoded_path(&uri), Some(format!("/{file}")));
    }

    #[test]
    fn a_diagnostic_publishes_zero_based_with_its_severity_and_help() {
        let published = lsp_diagnostic(&json!({
            "file": "a.astro", "line": 4, "column": 9, "endLine": 4, "endColumn": 13,
            "severity": "hint", "code": "ts(6385)", "message": "gone", "help": "Use variant.",
        }));
        assert_eq!(
            published,
            json!({
                "range": {
                    "start": {"line": 3, "character": 8},
                    "end": {"line": 3, "character": 12},
                },
                "severity": 4,
                "code": "ts(6385)",
                "source": "veneer",
                "message": "gone\nhelp: Use variant.",
            })
        );
    }

    #[test]
    fn a_diagnostic_without_help_keeps_its_message_and_markup_is_read() {
        let plain = lsp_diagnostic(&json!({"severity": "error", "message": "bad"}));
        assert_eq!(plain["message"], "bad");
        assert_eq!(plain["severity"], 1);
        let markup = lsp_diagnostic(&json!({"message": {"kind": "markdown", "value": "bad"}}));
        assert_eq!(markup["message"], "bad");
    }

    #[test]
    fn the_worker_option_is_read_wherever_it_stands() {
        let args = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
        let of = |list: &[&str]| worker_option(args(list).into_iter());
        assert_eq!(of(&["lsp", "--stdio"]), None);
        assert_eq!(of(&["lsp", "--worker"]), None);
        assert_eq!(
            of(&["lsp", "--stdio", "--worker", "w.mjs"]).as_deref(),
            Some("w.mjs")
        );
    }
}
