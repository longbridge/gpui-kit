use crate::engine::quickjs::js_error;
use std::{
    collections::VecDeque,
    net::TcpStream,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};

use rquickjs::{
    Ctx, FromJs, IntoJs, Object, Promise, Result, TypedArray, Value,
    function::{Func, Opt},
    module::{Declarations, Exports, ModuleDef},
};
use tungstenite::{
    HandshakeError, Message, WebSocket,
    client::IntoClientRequest,
    http::{HeaderMap, HeaderName, HeaderValue},
    protocol::WebSocketConfig,
    stream::MaybeTlsStream,
};

use super::{
    super::{host, scheduler},
    connect as network_connect,
};

const MESSAGE_LIMIT: usize = 8 * 1024 * 1024;
const COMMAND_QUEUE_LIMIT: usize = 8;
const INCOMING_QUEUE_LIMIT: usize = 8;
const READ_SLICE: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const IO_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(test)]
const IO_TIMEOUT: Duration = Duration::from_millis(100);

pub(super) struct WebSocketModule;

impl ModuleDef for WebSocketModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("WebSocket")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let websocket = Object::new(ctx.clone())?;
        websocket.set("connect", Func::from(connect))?;
        exports.export("WebSocket", websocket)?;
        Ok(())
    }
}

#[derive(Default)]
struct ConnectOptions {
    headers: HeaderMap,
}

impl<'js> FromJs<'js> for ConnectOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.is_null() || value.is_undefined() {
            return Ok(Self::default());
        }
        let Some(object) = value.into_object() else {
            return Err(js_error::type_error(
                ctx,
                "WebSocket.connect(url, options) expects an object with headers",
            ));
        };
        for key in object.keys::<String>() {
            let key = key?;
            if key != "headers" {
                return Err(js_error::type_error(
                    ctx,
                    &format!(
                        "unknown option `{key}` for WebSocket.connect(url, options); expected headers"
                    ),
                ));
            }
        }
        Ok(Self {
            headers: parse_headers(ctx, object.get("headers")?)?,
        })
    }
}

fn parse_headers<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<HeaderMap> {
    if value.is_null() || value.is_undefined() {
        return Ok(HeaderMap::new());
    }
    let Some(object) = value.into_object() else {
        return Err(js_error::type_error(
            ctx,
            "WebSocket.connect(url, options).headers expects a plain object of string values",
        ));
    };
    let mut headers = HeaderMap::new();
    for entry in object.props::<String, Value>() {
        let (name, value) = entry?;
        let normalized = name.to_ascii_lowercase();
        if is_forbidden_handshake_header(&normalized) {
            return Err(js_error::type_error(
                ctx,
                &format!(
                    "WebSocket.connect(url, options).headers may not set handshake-control or credential header `{name}`"
                ),
            ));
        }
        let name = HeaderName::from_bytes(normalized.as_bytes()).map_err(|_| {
            js_error::type_error(ctx, "WebSocket.connect received an invalid header name")
        })?;
        let value = String::from_js(ctx, value).map_err(|_| {
            js_error::type_error(
                ctx,
                "WebSocket.connect(url, options).headers expects string header values",
            )
        })?;
        let value = HeaderValue::from_str(&value).map_err(|_| {
            js_error::type_error(ctx, "WebSocket.connect received an invalid header value")
        })?;
        headers.append(name, value);
    }
    Ok(headers)
}

fn is_forbidden_handshake_header(name: &str) -> bool {
    matches!(
        name,
        "authorization" | "proxy-authorization" | "cookie" | "host" | "connection" | "upgrade"
    ) || name.starts_with("sec-websocket-")
}

fn connect<'js>(ctx: Ctx<'js>, url: String, options: Opt<ConnectOptions>) -> Result<Promise<'js>> {
    let parsed = reqwest::Url::parse(&url)
        .map_err(|error| js_error::type_error(&ctx, &format!("invalid WebSocket URL: {error}")))?;
    if !matches!(parsed.scheme(), "ws" | "wss") {
        return Err(js_error::type_error(
            &ctx,
            "WebSocket.connect(url) expects a ws:// or wss:// URL",
        ));
    }
    let host_name = parsed
        .host_str()
        .ok_or_else(|| js_error::type_error(&ctx, "WebSocket URL has no host"))?
        .to_owned();
    if !host::capabilities().may_reach(&host_name) {
        return Err(js_error::type_error(
            &ctx,
            &format!(
                "network access to `{host_name}` is not granted; add it to capabilities.network.hosts"
            ),
        ));
    }
    let port = parsed.port_or_known_default().ok_or_else(|| {
        js_error::type_error(&ctx, "WebSocket URL has no port and no known default")
    })?;
    let headers = options.0.unwrap_or_default().headers;
    scheduler::blocking(&ctx, "WebSocket.connect(url)", move || {
        let config = WebSocketConfig::default()
            .max_message_size(Some(MESSAGE_LIMIT))
            .max_frame_size(Some(MESSAGE_LIMIT));
        let mut request = url
            .into_client_request()
            .map_err(|error| format!("WebSocket handshake failed: {error}"))?;
        request.headers_mut().extend(headers);
        request
            .headers_mut()
            .entry(tungstenite::http::header::USER_AGENT)
            .or_insert(HeaderValue::from_static(super::USER_AGENT));
        let operation = format!("WebSocket host {host_name}:{port}");
        let (stream, deadline) =
            network_connect::connect_tcp(&host_name, port, IO_TIMEOUT, &operation)
                .map_err(|error| format!("WebSocket connection failed: {error}"))?;
        let handshake_timeout =
            network_connect::remaining_io_timeout(deadline, "WebSocket handshake")?;
        stream
            .set_read_timeout(Some(handshake_timeout))
            .map_err(|error| format!("setting WebSocket handshake timeout failed: {error}"))?;
        stream
            .set_write_timeout(Some(handshake_timeout))
            .map_err(|error| format!("setting WebSocket handshake timeout failed: {error}"))?;
        let (mut socket, _) =
            tungstenite::client_tls_with_config(request, stream, Some(config), None).map_err(
                |error| match error {
                    HandshakeError::Interrupted(_) => "WebSocket handshake timed out".to_owned(),
                    HandshakeError::Failure(tungstenite::Error::Io(error))
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) =>
                    {
                        "WebSocket handshake timed out".to_owned()
                    }
                    other => format!("WebSocket handshake failed: {other}"),
                },
            )?;
        set_timeouts(&mut socket)?;
        Socket::new(socket)
    })
}

struct Socket {
    commands: SyncSender<Command>,
    read_outstanding: Arc<AtomicBool>,
}

enum IncomingMessage {
    Text(String),
    Binary(Vec<u8>),
}

enum ReadOutcome {
    Message(IncomingMessage),
    TimedOut,
}

enum Command {
    Read(PendingRead),
    Write(Message, scheduler::ActorCompletion),
    Close(scheduler::ActorCompletion),
}

struct PendingRead {
    reply: scheduler::ActorCompletion<IncomingMessage>,
    outstanding: Arc<AtomicBool>,
}

impl PendingRead {
    fn settle(self, result: std::result::Result<IncomingMessage, String>) {
        self.outstanding.store(false, Ordering::Release);
        self.reply.settle(result);
    }
}

impl<'js> IntoJs<'js> for IncomingMessage {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            Self::Text(text) => text.into_js(ctx),
            Self::Binary(bytes) => TypedArray::<u8>::new(ctx.clone(), bytes)?.into_js(ctx),
        }
    }
}

fn outgoing_message<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Message> {
    if value.is_string() {
        let text = String::from_js(ctx, value)?;
        if text.len() > MESSAGE_LIMIT {
            return Err(js_error::range_error(
                ctx,
                &format!("WebSocket write exceeded the {MESSAGE_LIMIT} byte limit"),
            ));
        }
        return Ok(Message::Text(text.into()));
    }

    let bytes = TypedArray::<u8>::from_js(ctx, value).map_err(|_| {
        js_error::type_error(ctx, "WebSocket.write(data) expects a string or Uint8Array")
    })?;
    // SAFETY: finish copying before error creation or other JavaScript can run.
    let bytes = unsafe { bytes.as_bytes().map(<[u8]>::to_vec) }.ok_or_else(|| {
        js_error::type_error(ctx, "WebSocket.write(data) received a detached Uint8Array")
    })?;
    if bytes.len() > MESSAGE_LIMIT {
        return Err(js_error::range_error(
            ctx,
            &format!("WebSocket write exceeded the {MESSAGE_LIMIT} byte limit"),
        ));
    }
    Ok(Message::Binary(bytes.into()))
}

fn set_timeouts(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
) -> std::result::Result<(), String> {
    let stream = match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => &mut stream.sock,
        _ => return Err("WebSocket transport does not support read timeouts".to_owned()),
    };
    stream
        .set_read_timeout(Some(READ_SLICE))
        .map_err(|error| format!("setting WebSocket read timeout failed: {error}"))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| format!("setting WebSocket write timeout failed: {error}"))
}

impl Socket {
    fn new(mut socket: WebSocket<MaybeTlsStream<TcpStream>>) -> std::result::Result<Self, String> {
        set_timeouts(&mut socket)?;
        let (commands, receiver) = mpsc::sync_channel(COMMAND_QUEUE_LIMIT);
        thread::Builder::new()
            .name("gpui-websocket".to_owned())
            .spawn(move || run_actor(socket, receiver))
            .map_err(|error| format!("starting WebSocket actor failed: {error}"))?;
        Ok(Self {
            commands,
            read_outstanding: Arc::new(AtomicBool::new(false)),
        })
    }
}

fn reject_command(command: Command, error: String) {
    match command {
        Command::Read(reply) => reply.settle(Err(error)),
        Command::Write(_, reply) | Command::Close(reply) => reply.settle(Err(error)),
    }
}

fn enqueue(commands: &SyncSender<Command>, command: Command) {
    match commands.try_send(command) {
        Ok(()) => {}
        Err(TrySendError::Full(command)) => reject_command(
            command,
            "WebSocket command queue is full; wait for an outstanding operation".to_owned(),
        ),
        Err(TrySendError::Disconnected(command)) => {
            reject_command(command, "WebSocket connection is closed".to_owned())
        }
    }
}

fn fail_reads(reads: &mut VecDeque<PendingRead>, error: String) {
    while let Some(reply) = reads.pop_front() {
        reply.settle(Err(error.clone()));
    }
}

fn operation_error(operation: &str, error: tungstenite::Error) -> String {
    match error {
        tungstenite::Error::Io(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            format!("WebSocket {operation} timed out")
        }
        other => format!("WebSocket {operation} failed: {other}"),
    }
}

fn handle_command(
    command: Command,
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    reads: &mut VecDeque<PendingRead>,
    messages: &mut VecDeque<IncomingMessage>,
) -> bool {
    match command {
        Command::Read(reply) => {
            if let Some(message) = messages.pop_front() {
                reply.settle(Ok(message));
            } else {
                reads.push_back(reply);
            }
            true
        }
        Command::Write(message, reply) => {
            let result = socket
                .send(message)
                .map_err(|error| operation_error("write", error));
            let alive = result.is_ok();
            reply.settle(result);
            if !alive {
                fail_reads(reads, "WebSocket connection is closed".to_owned());
            }
            alive
        }
        Command::Close(reply) => {
            let result = socket
                .close(None)
                .and_then(|_| socket.flush())
                .map_err(|error| operation_error("close", error));
            let error = result.as_ref().err().cloned();
            reply.settle(result);
            fail_reads(
                reads,
                error.unwrap_or_else(|| "WebSocket closed by this endpoint".to_owned()),
            );
            false
        }
    }
}

fn run_actor(mut socket: WebSocket<MaybeTlsStream<TcpStream>>, receiver: Receiver<Command>) {
    let mut reads = VecDeque::new();
    let mut messages = VecDeque::new();
    loop {
        if reads.is_empty() {
            match receiver.recv_timeout(READ_SLICE) {
                Ok(command) => {
                    if !handle_command(command, &mut socket, &mut reads, &mut messages) {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        loop {
            match receiver.try_recv() {
                Ok(command) => {
                    if !handle_command(command, &mut socket, &mut reads, &mut messages) {
                        return;
                    }
                }
                Err(TryRecvError::Disconnected) => return,
                Err(TryRecvError::Empty) => break,
            }
        }

        match read_message(&mut socket) {
            Ok(ReadOutcome::Message(message)) => {
                if let Some(reply) = reads.pop_front() {
                    reply.settle(Ok(message));
                } else if messages.len() < INCOMING_QUEUE_LIMIT {
                    messages.push_back(message);
                } else {
                    let _ = socket.close(None);
                    return;
                }
            }
            Ok(ReadOutcome::TimedOut) => {}
            Err(error) => {
                fail_reads(&mut reads, error);
                return;
            }
        }
    }
}

fn read_message(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
) -> std::result::Result<ReadOutcome, String> {
    loop {
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return Ok(ReadOutcome::TimedOut);
            }
            Err(error) => return Err(format!("WebSocket read failed: {error}")),
        };
        match message {
            Message::Text(text) => {
                return Ok(ReadOutcome::Message(IncomingMessage::Text(
                    text.to_string(),
                )));
            }
            Message::Binary(bytes) => {
                return Ok(ReadOutcome::Message(IncomingMessage::Binary(
                    bytes.to_vec(),
                )));
            }
            Message::Ping(_) => socket
                .flush()
                .map_err(|error| format!("WebSocket pong failed: {error}"))?,
            Message::Pong(_) => {}
            Message::Close(_) => {
                socket
                    .flush()
                    .map_err(|error| format!("WebSocket close reply failed: {error}"))?;
                return Err("WebSocket closed by peer".to_owned());
            }
            Message::Frame(_) => {}
        }
    }
}

impl<'js> IntoJs<'js> for Socket {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let socket = Object::new(ctx.clone())?;
        let writer = self.commands.clone();
        socket.set(
            "write",
            Func::from(
                move |ctx: Ctx<'js>, value: Value<'js>| -> Result<Promise<'js>> {
                    let message = outgoing_message(&ctx, value)?;
                    let commands = writer.clone();
                    let (promise, reply) =
                        scheduler::actor_deferred(&ctx, "WebSocket.write(data)")?;
                    enqueue(&commands, Command::Write(message, reply));
                    Ok(promise)
                },
            ),
        )?;
        let reader = self.commands.clone();
        let read_outstanding = self.read_outstanding;
        socket.set(
            "read",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                if read_outstanding
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    return Err(js_error::type_error(
                        &ctx,
                        "WebSocket.read() already has an outstanding read",
                    ));
                }
                let commands = reader.clone();
                let (promise, reply) = match scheduler::actor_blocking(&ctx, "WebSocket.read()") {
                    Ok(deferred) => deferred,
                    Err(error) => {
                        read_outstanding.store(false, Ordering::Release);
                        return Err(error);
                    }
                };
                let pending = PendingRead {
                    reply,
                    outstanding: read_outstanding.clone(),
                };
                enqueue(&commands, Command::Read(pending));
                Ok(promise)
            }),
        )?;
        let closer = self.commands;
        socket.set(
            "close",
            Func::from(move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                let commands = closer.clone();
                let (promise, reply) = scheduler::actor_deferred(&ctx, "WebSocket.close()")?;
                enqueue(&commands, Command::Close(reply));
                Ok(promise)
            }),
        )?;
        Ok(socket.into_value())
    }
}
