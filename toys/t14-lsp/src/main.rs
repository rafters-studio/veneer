// T14: LSP on stdio in front of one Node worker holding a warm Project, JSON lines between them.
use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, Notification as _, PublishDiagnostics};
use lsp_types::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Instant;

enum Event { Lsp(Message), Worker(Value) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dist, root, folder, worker) = (&args[1], &args[2], &args[3], &args[4]);
    let (connection, _threads) = Connection::stdio();
    let caps = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    }).unwrap();
    connection.initialize(caps).unwrap();

    let mut child = Command::new("node")
        .args([worker, dist, root, folder])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
        .spawn().unwrap();
    let mut to_worker = child.stdin.take().unwrap();
    let from_worker = BufReader::new(child.stdout.take().unwrap());

    let (tx, rx) = mpsc::channel::<Event>();
    let tx2 = tx.clone();
    std::thread::spawn(move || {
        for line in from_worker.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str(&line) { let _ = tx2.send(Event::Worker(value)); }
        }
    });
    let receiver = connection.receiver.clone();
    std::thread::spawn(move || { for m in receiver { if tx.send(Event::Lsp(m)).is_err() { break; } } });

    // One request in flight per server; newest text wins, so a burst cannot publish stale results.
    let mut pending: Option<(i64, String, String)> = None;
    let mut inflight: Option<(i64, String, Instant)> = None;
    let mut latest: HashMap<String, i64> = HashMap::new();
    let mut next_seq = 0i64;
    let mut ready = false;
    let rootp = root.trim_end_matches('/').to_string();

    let send = |to: &mut std::process::ChildStdin, seq: i64, file: &str, text: &str| {
        let line = json!({"seq": seq, "file": file, "text": text}).to_string();
        writeln!(to, "{line}").unwrap();
        to.flush().unwrap();
    };

    for event in rx {
        match event {
            Event::Lsp(Message::Request(req)) => {
                if connection.handle_shutdown(&req).unwrap() { break; }
                connection.sender.send(Message::Response(Response::new_ok(req.id, Value::Null))).unwrap();
            }
            Event::Lsp(Message::Notification(n)) => {
                let doc = match n.method.as_str() {
                    DidOpenTextDocument::METHOD => {
                        let p: DidOpenTextDocumentParams = serde_json::from_value(n.params).unwrap();
                        Some((p.text_document.uri, p.text_document.text))
                    }
                    DidChangeTextDocument::METHOD => {
                        let mut p: DidChangeTextDocumentParams = serde_json::from_value(n.params).unwrap();
                        p.content_changes.pop().map(|c| (p.text_document.uri, c.text))
                    }
                    _ => None,
                };
                if let Some((uri, text)) = doc {
                    let path = uri.path().as_str().to_string();
                    let file = path.strip_prefix(&format!("{rootp}/")).unwrap_or(&path).to_string();
                    next_seq += 1;
                    latest.insert(file.clone(), next_seq);
                    if ready && inflight.is_none() {
                        send(&mut to_worker, next_seq, &file, &text);
                        inflight = Some((next_seq, uri.to_string(), Instant::now()));
                    } else {
                        pending = Some((next_seq, uri.to_string(), text));
                    }
                }
            }
            Event::Lsp(_) => {}
            Event::Worker(v) => {
                if v.get("ready").is_some() { ready = true; }
                else if let Some((seq, uri, started)) = inflight.take() {
                    let file = v["file"].as_str().unwrap_or("").to_string();
                    let stale = latest.get(&file).is_some_and(|l| *l != seq);
                    if !stale {
                        let diagnostics: Vec<Value> = v["diagnostics"].as_array().cloned().unwrap_or_default().iter().map(|d| {
                            let help = d["help"].as_str().map(|h| format!("\nhelp: {h}")).unwrap_or_default();
                            json!({
                                "range": {"start": {"line": d["line"].as_u64().unwrap_or(1) - 1, "character": d["column"].as_u64().unwrap_or(1) - 1},
                                          "end": {"line": d["endLine"].as_u64().unwrap_or(1) - 1, "character": d["endColumn"].as_u64().unwrap_or(1) - 1}},
                                "severity": match d["severity"].as_str() { Some("warning") => 2, Some("info") => 3, Some("hint") => 4, _ => 1 },
                                "code": d["code"], "source": "veneer",
                                "message": format!("{}{help}", d["message"].as_str().unwrap_or("")),
                            })
                        }).collect();
                        let params = json!({"uri": uri, "diagnostics": diagnostics,
                            "_t14": {"seq": seq, "workerMs": v["workerMs"], "serverMs": started.elapsed().as_secs_f64() * 1000.0, "reopened": v["reopened"], "outside": v["outside"], "error": v["error"]}});
                        connection.sender.send(Message::Notification(Notification::new(PublishDiagnostics::METHOD.to_string(), params))).unwrap();
                    }
                }
                if inflight.is_none() {
                    if let Some((seq, uri, text)) = pending.take() {
                        let parsed: Uri = uri.parse().unwrap();
                        let path = parsed.path().as_str().to_string();
                        let file = path.strip_prefix(&format!("{rootp}/")).unwrap_or(&path).to_string();
                        send(&mut to_worker, seq, &file, &text);
                        inflight = Some((seq, uri, Instant::now()));
                    }
                }
            }
        }
    }
    drop(to_worker);
    let _ = child.wait();
    drop(connection);
}
