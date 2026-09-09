use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    ops::Deref,
    sync::mpsc,
    thread,
    time::Duration,
};

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};
use tungstenite::Message;

use crate::{Capabilities, ScriptView, ShellRuntime};

const FETCH_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const response = await fetch("__URL__");
        this.state = `${response.status}|${response.ok}|${await response.text()}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const NET_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { connect } from "net";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const socket = await connect("127.0.0.1", __PORT__);
        await socket.write("ping");
        const bytes = await socket.read(4);
        const eof = await socket.read(4);
        this.state = `${bytes instanceof Uint8Array}|${[...bytes].join(",")}|${eof === null}`;
        socket.close();
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const NET_LIMIT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { connect } from "net";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const socket = await connect("127.0.0.1", __PORT__);
      const errors = [];
      try { await socket.write("x".repeat(1048577)); }
      catch (error) { errors.push(error.message); }
      try { await socket.read(1048577); }
      catch (error) { errors.push(error.message); }
      socket.close();
      this.state = errors.join("|");
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const NET_PENDING_READ_CLOSE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { connect } from "net";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const socket = await connect("127.0.0.1", __PORT__);
      const reading = socket.read(1).catch(() => undefined);
      await cx.sleep(50);
      socket.close();
      await reading;
      this.state = "closed";
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const standardGlobal = typeof globalThis.WebSocket;
      try {
        await WebSocket.connect("wss://quotes.example.test/v2");
        this.state = "connected";
      } catch (error) {
        this.state = `${standardGlobal}|rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_HANDSHAKE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await WebSocket.connect("__URL__");
        this.state = "connected";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_HEADERS_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        await WebSocket.connect("__URL__", { headers: __HEADERS__ });
        this.state = "connected";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_MESSAGES_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const socket = await WebSocket.connect("__URL__");
        await socket.write("client text");
        await socket.write(new Uint8Array([1, 2, 3]));
        const receivedText = await socket.read();
        const receivedBytes = await socket.read();
        this.state = `${receivedText}|${receivedBytes instanceof Uint8Array}|${[...receivedBytes].join(",")}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_PENDING_READ_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const socket = await WebSocket.connect("__URL__");
        const reading = socket.read().catch(() => undefined);
        await socket.write("while read is pending");
        await socket.close();
        await reading;
        this.state = "write-and-close-finished";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_IDLE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      this.socket = await WebSocket.connect("__URL__");
      this.state = "connected";
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_CLOSED_WRITE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const socket = await WebSocket.connect("__URL__");
        try { await socket.read(); } catch (_) {}
        await socket.write("after close");
        this.state = "write-resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_CONCURRENT_READ_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const socket = await WebSocket.connect("__URL__");
      const first = socket.read().catch(() => undefined);
      try {
        await socket.read();
        this.state = "second-read-resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      await socket.close();
      await first;
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_STALLED_WRITE_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      try {
        const socket = await WebSocket.connect("__URL__");
        const payload = "x".repeat(8 * 1024 * 1024);
        for (let attempt = 0; attempt < 32; attempt += 1) {
          await socket.write(payload);
        }
        this.state = "write-resolved";
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      cx.notify();
    });
  }
  render() { return v_flex().child(this.state); }
}
"#;

const WEBSOCKET_QUEUE_LIMIT_PROBE: &str = r#"
import { div, View } from "gpui-kit";
import { v_flex } from "gpui-base";
import { WebSocket } from "websocket";

export default class Probe extends View {
  init(_props, cx) {
    this.state = "pending";
    cx.spawn(async (cx) => {
      const socket = await WebSocket.connect("__URL__");
      const payload = "x".repeat(2 * 1024 * 1024);
      const writes = [];
      for (let index = 0; index < 16; index += 1) {
        writes.push(socket.write(payload));
      }
      let close;
      try {
        close = socket.close();
      } catch (error) {
        this.state = `synchronous-close:${error.message}`;
        cx.notify();
        return;
      }
      const outcomes = await Promise.allSettled([...writes, close]);
      const errors = outcomes
        .filter((outcome) => outcome.status === "rejected")
        .map((outcome) => outcome.reason.message);
      let reaped = "reaped";
      try {
        for (let batch = 0; batch < 80; batch += 1) {
          const rejected = [];
          for (let index = 0; index < 16; index += 1) {
            rejected.push(socket.write("after-close"));
          }
          await Promise.allSettled(rejected);
        }
      } catch (error) {
        reaped = `leaked:${error.message}`;
      }
      this.state = `${errors.join("|")}|${reaped}`;
      cx.notify();
    });
  }
  render(cx) { return v_flex().child(this.state); }
}
"#;

#[gpui::test]
fn fetch_runs_off_thread_and_obeys_the_active_policy(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
            .expect("HTTP response");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/probe"));
    let (_runtime, view, mut context) = probe(cx, &source);
    draw(&mut context, &view);
    assert!(snapshot(&mut context, &view).contains("pending"));
    context.run_until_parked();
    draw(&mut context, &view);
    assert!(
        snapshot(&mut context, &view).contains("200|true|hello"),
        "fetch did not settle through the script boundary"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn net_connect_is_bounded_and_capability_gated(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("TCP listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("TCP connection");
        let mut request = [0; 4];
        stream.read_exact(&mut request).expect("read ping");
        assert_eq!(&request, b"ping");
        stream.write_all(b"pong").expect("write pong");
    });

    let source = NET_PROBE.replace("__PORT__", &port.to_string());
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    assert!(
        snapshot(&mut context, &view).contains("true|112,111,110,103|true"),
        "raw TCP reads should preserve bytes and report EOF"
    );
    server.join().expect("TCP server");
}

#[gpui::test]
fn net_close_does_not_wait_for_a_pending_read(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("TCP listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("TCP connection");
        thread::sleep(Duration::from_millis(300));
        let _ = stream.write_all(b"x");
    });

    let source = NET_PENDING_READ_CLOSE_PROBE.replace("__PORT__", &port.to_string());
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    thread::sleep(Duration::from_millis(20));
    context.executor().advance_clock(Duration::from_millis(50));
    let started = std::time::Instant::now();
    context.run_until_parked();
    let close_elapsed = started.elapsed();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);

    server.join().expect("TCP server");
    assert!(
        close_elapsed < Duration::from_millis(150),
        "socket.close() waited {close_elapsed:?} for the pending read"
    );
    assert!(rendered.contains("closed"), "{rendered}");
}

#[gpui::test]
fn network_is_denied_by_default(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = FETCH_PROBE.replace("__URL__", "http://127.0.0.1:9/");
    let view_type = runtime
        .load_source("denied-fetch.js", &source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("the async task catches the denial");
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && rendered.contains("capabilities.network.hosts"),
        "{rendered}"
    );
}

#[gpui::test]
fn net_connect_is_denied_by_default(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let source = NET_PROBE.replace("__PORT__", "9");
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("denied-net.js", &source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("the async task catches the denial");
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && rendered.contains("capabilities.network.hosts"),
        "{rendered}"
    );
}

#[gpui::test]
fn websocket_is_denied_by_default(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("denied-websocket.js", WEBSOCKET_PROBE)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("the async task catches the denial");
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("undefined|rejected:") && rendered.contains("capabilities.network.hosts"),
        "{rendered}"
    );
}

#[gpui::test]
fn websocket_performs_a_real_handshake_off_thread(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("WebSocket connection");
        let mut request = [0; 2048];
        let count = stream.read(&mut request).expect("handshake request");
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.starts_with("GET /v2 HTTP/1.1"), "{request}");
        assert!(request.to_ascii_lowercase().contains("upgrade: websocket"));
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .expect("handshake rejection");
    });

    let source = WEBSOCKET_HANDSHAKE_PROBE.replace("__URL__", &format!("ws://{address}/v2"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && !rendered.contains("pending"),
        "WebSocket.connect must wait for and report the server handshake: {rendered}"
    );
    server.join().expect("WebSocket server");
}

#[gpui::test]
fn websocket_sends_ordinary_and_custom_protocol_headers(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let mut stream = (0..50)
            .find_map(|_| match listener.accept() {
                Ok((stream, _)) => Some(stream),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    None
                }
                Err(error) => panic!("WebSocket accept failed: {error}"),
            })
            .expect("WebSocket connection");
        let mut request = [0; 4096];
        let count = stream.read(&mut request).expect("handshake request");
        let request = String::from_utf8_lossy(&request[..count]).to_ascii_lowercase();
        assert!(request.contains("accept-language: zh-cn\r\n"), "{request}");
        assert!(
            request.contains("user-agent: protocol-client/1\r\n"),
            "{request}"
        );
        assert!(request.contains("x-protocol-region: us\r\n"), "{request}");
        stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .expect("handshake rejection");
    });

    let source = WEBSOCKET_HEADERS_PROBE
        .replace("__URL__", &format!("ws://{address}/headers"))
        .replace(
            "__HEADERS__",
            r#"{
              "Accept-Language": "zh-CN",
              "User-Agent": "protocol-client/1",
              "X-Protocol-Region": "us"
            }"#,
        );
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    assert!(snapshot(&mut context, &view).contains("rejected:"));
    server.join().expect("WebSocket server");
}

#[gpui::test]
fn websocket_rejects_sensitive_handshake_headers_before_connecting(cx: &mut TestAppContext) {
    for header in [
        "authorization",
        "proxy-authorization",
        "cookie",
        "host",
        "connection",
        "upgrade",
        "sec-websocket-protocol",
        "sec-websocket-extensions",
        "sec-websocket-version",
        "sec-websocket-key",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let address = listener.local_addr().expect("listener address");
        let source = WEBSOCKET_HEADERS_PROBE
            .replace("__URL__", &format!("ws://{address}/denied"))
            .replace("__HEADERS__", &format!(r#"{{ "{header}": "secret" }}"#));
        let (_runtime, view, mut context) = probe(cx, &source);
        context.run_until_parked();
        draw(&mut context, &view);
        let rendered = snapshot(&mut context, &view);
        assert!(
            rendered.contains("rejected:") && rendered.contains(header),
            "{rendered}"
        );
        assert_eq!(
            listener
                .accept()
                .expect_err("a rejected header must not reach the network")
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[gpui::test]
fn websocket_redirect_does_not_escape_the_authorized_host(cx: &mut TestAppContext) {
    let redirector = TcpListener::bind("127.0.0.1:0").expect("authorized WebSocket listener");
    let redirect_address = redirector.local_addr().expect("redirect listener address");
    let target = TcpListener::bind("127.0.0.1:0").expect("redirect target listener");
    let target_port = target.local_addr().expect("target listener address").port();

    let redirect_server = thread::spawn(move || {
        let (mut stream, _) = redirector
            .accept()
            .expect("authorized WebSocket connection");
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).expect("handshake request");
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: ws://localhost:{target_port}/denied\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .expect("redirect response");
    });
    let target_server = thread::spawn(move || {
        target
            .set_nonblocking(true)
            .expect("nonblocking redirect target");
        for _ in 0..50 {
            match target.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0; 2048];
                    let _ = stream.read(&mut request);
                    let _ = stream.write_all(
                        b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    return true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("redirect target accept failed: {error}"),
            }
        }
        false
    });

    let source =
        WEBSOCKET_HANDSHAKE_PROBE.replace("__URL__", &format!("ws://{redirect_address}/redirect"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(rendered.contains("rejected:"), "{rendered}");
    redirect_server.join().expect("redirect server");
    assert!(
        !target_server.join().expect("redirect target server"),
        "WebSocket.connect followed a redirect to an unauthorized host"
    );
}

#[gpui::test]
fn websocket_reads_and_writes_text_and_binary_messages(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (progress, progress_receiver) = mpsc::channel();
    // Closing rejects a read that has not completed yet, so the server waits to
    // be told the client has the messages instead of closing straight after
    // sending them. Without this the test is a race the client loses whenever
    // the runner is slow enough, and the symptom — a rejected read — looks like
    // a product failure rather than a starved thread.
    let (may_close, close_permission) = mpsc::channel::<()>();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        let mut socket = tungstenite::accept(stream).expect("WebSocket handshake");
        assert_eq!(
            socket.read().expect("client text"),
            Message::Text("client text".into())
        );
        let _ = progress.send(());
        assert_eq!(
            socket.read().expect("client binary"),
            Message::Binary(vec![1, 2, 3].into())
        );
        socket
            .send(Message::Text("server text".into()))
            .expect("server text");
        socket
            .send(Message::Binary(vec![4, 5, 6].into()))
            .expect("server binary");
        let _ = progress.send(());
        let _ = close_permission.recv_timeout(Duration::from_secs(10));
        socket.close(None).expect("server close");
        socket.flush().expect("flush server close");
    });

    let source = WEBSOCKET_MESSAGES_PROBE.replace("__URL__", &format!("ws://{address}/messages"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    progress_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("first client write");
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        context.executor().advance_clock(Duration::from_millis(10));
        context.run_until_parked();
        match progress_receiver.try_recv() {
            Ok(()) => break,
            Err(mpsc::TryRecvError::Empty) if std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("server messages after second client write: {error}"),
        }
    }
    // Poll for the messages rather than pumping a fixed number of times: how
    // many turns the client needs is the runner's business, not this test's.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let rendered = loop {
        draw(&mut context, &view);
        let rendered = snapshot(&mut context, &view);
        if rendered.contains("server text|true|4,5,6") {
            break rendered;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for the server messages: {rendered}"
        );
        thread::sleep(Duration::from_millis(10));
        context.executor().advance_clock(Duration::from_millis(10));
        context.run_until_parked();
    };
    let _ = may_close.send(());
    let _ = server.join().expect("WebSocket server");
    assert!(rendered.contains("server text|true|4,5,6"), "{rendered}");
}

#[gpui::test]
fn websocket_pending_read_does_not_block_write_or_close(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (text_received, text_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("server read timeout");
        let mut socket = tungstenite::accept(stream).expect("WebSocket handshake");
        socket
            .send(Message::Ping(vec![7].into()))
            .expect("server ping");

        let mut saw_pong = false;
        let mut saw_text = false;
        let mut saw_close = false;
        for _ in 0..3 {
            match socket.read() {
                Ok(Message::Pong(bytes)) if bytes.as_ref() == [7] => saw_pong = true,
                Ok(Message::Text(text)) if text == "while read is pending" => {
                    saw_text = true;
                    let _ = text_received.send(());
                }
                Ok(message) if message.is_close() => saw_close = true,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        (saw_pong, saw_text, saw_close)
    });

    let source =
        WEBSOCKET_PENDING_READ_PROBE.replace("__URL__", &format!("ws://{address}/pending"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    text_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("client write while read is pending");
    context.executor().advance_clock(Duration::from_millis(10));
    context.run_until_parked();
    let server_events = server.join().expect("WebSocket server");
    context.executor().advance_clock(Duration::from_millis(10));
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert_eq!(server_events, (true, true, true));
    assert!(rendered.contains("write-and-close-finished"), "{rendered}");
}

#[gpui::test]
fn websocket_answers_ping_while_javascript_has_no_pending_read(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (connected, connected_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("server read timeout");
        let mut socket = tungstenite::accept(stream).expect("WebSocket handshake");
        let _ = connected.send(());
        socket
            .send(Message::Ping(vec![9].into()))
            .expect("server ping");
        match socket.read() {
            Ok(Message::Pong(bytes)) => assert_eq!(bytes.as_ref(), [9]),
            other => panic!("idle client did not answer ping: {other:?}"),
        }
    });

    let source = WEBSOCKET_IDLE_PROBE.replace("__URL__", &format!("ws://{address}/idle"));
    let (_runtime, _view, context) = probe(cx, &source);
    context.run_until_parked();
    connected_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("client connected");

    server.join().expect("WebSocket server");
}

#[gpui::test]
fn websocket_connect_rejects_when_the_server_stalls_the_handshake(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (accepted, accepted_receiver) = mpsc::channel();
    let (release, release_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (_stream, _) = listener.accept().expect("WebSocket connection");
        accepted.send(()).expect("accepted signal");
        let _ = release_receiver.recv_timeout(Duration::from_secs(1));
    });

    let source = WEBSOCKET_HANDSHAKE_PROBE.replace("__URL__", &format!("ws://{address}/stalled"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    accepted_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("client connected to the stalled server");
    thread::sleep(Duration::from_millis(200));
    context.executor().advance_clock(Duration::from_millis(200));
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);

    let _ = release.send(());
    server.join().expect("WebSocket server");
    assert!(
        rendered.contains("rejected:") && rendered.contains("timed out"),
        "a stalled WebSocket handshake must reject before the server is released: {rendered}"
    );
}

#[gpui::test]
fn websocket_rejects_a_second_outstanding_read(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        let mut socket = tungstenite::accept(stream).expect("WebSocket handshake");
        let _ = socket.read();
    });

    let source =
        WEBSOCKET_CONCURRENT_READ_PROBE.replace("__URL__", &format!("ws://{address}/reads"));
    let (_runtime, view, mut context) = probe(cx, &source);
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(10));
        context.executor().advance_clock(Duration::from_millis(10));
        context.run_until_parked();
    }
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:WebSocket.read() already has an outstanding read"),
        "{rendered}"
    );
    server.join().expect("WebSocket server");
}

#[gpui::test]
fn websocket_write_rejects_when_the_peer_stops_reading(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (accepted, accepted_receiver) = mpsc::channel();
    let (release, release_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        let _socket = tungstenite::accept(stream).expect("WebSocket handshake");
        accepted.send(()).expect("accepted signal");
        let _ = release_receiver.recv_timeout(Duration::from_secs(1));
    });

    let source =
        WEBSOCKET_STALLED_WRITE_PROBE.replace("__URL__", &format!("ws://{address}/stalled"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    accepted_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("WebSocket handshake completed");
    for _ in 0..12 {
        thread::sleep(Duration::from_millis(50));
        context.executor().advance_clock(Duration::from_millis(50));
        context.run_until_parked();
    }
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);

    let _ = release.send(());
    server.join().expect("WebSocket server");
    assert!(
        rendered.contains("rejected:WebSocket write timed out"),
        "a stalled WebSocket write must reject before the server is released: {rendered}"
    );
}

#[gpui::test]
fn websocket_large_writes_reject_when_the_command_queue_is_full(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let (accepted, accepted_receiver) = mpsc::channel();
    let (release, release_receiver) = mpsc::channel();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        let _socket = tungstenite::accept(stream).expect("WebSocket handshake");
        accepted.send(()).expect("accepted signal");
        let _ = release_receiver.recv_timeout(Duration::from_secs(2));
    });

    let source = WEBSOCKET_QUEUE_LIMIT_PROBE.replace("__URL__", &format!("ws://{address}/queue"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    accepted_receiver
        .recv_timeout(Duration::from_millis(500))
        .expect("WebSocket handshake completed");
    for _ in 0..20 {
        thread::sleep(Duration::from_millis(25));
        context.executor().advance_clock(Duration::from_millis(25));
        context.run_until_parked();
    }
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);

    let _ = release.send(());
    server.join().expect("WebSocket server");
    assert!(
        rendered.contains("WebSocket command queue is full")
            && rendered.contains("reaped")
            && !rendered.contains("synchronous-close:")
            && !rendered.contains("leaked:"),
        "large writes and close beyond the bounded queue must reject asynchronously without leaking tasks: {rendered}"
    );
}

#[gpui::test]
fn websocket_write_rejects_after_the_server_closes(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("WebSocket listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("WebSocket connection");
        let mut socket = tungstenite::accept(stream).expect("WebSocket handshake");
        socket.close(None).expect("queue close");
        socket.flush().expect("flush close");
    });

    let source = WEBSOCKET_CLOSED_WRITE_PROBE.replace("__URL__", &format!("ws://{address}/closed"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    server.join().expect("WebSocket server");
    thread::sleep(Duration::from_millis(10));
    context.executor().advance_clock(Duration::from_millis(10));
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:WebSocket connection is closed"),
        "{rendered}"
    );
}

#[gpui::test]
fn fetch_reauthorizes_every_redirect_target(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://localhost:{}/denied\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            address.port()
        );
        stream.write_all(response.as_bytes()).expect("redirect");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/redirect"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:")
            && rendered.contains("redirect target")
            && rendered.contains("localhost"),
        "{rendered}"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn fetch_rejects_a_response_over_the_buffer_limit(cx: &mut TestAppContext) {
    const TOO_LARGE: usize = 8 * 1024 * 1024 + 1;
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {TOO_LARGE}\r\nConnection: close\r\n\r\n"
        )
        .expect("headers");
        let body = vec![b'x'; TOO_LARGE];
        stream.write_all(&body).expect("oversized body");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/large"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && rendered.contains("8388608 byte limit"),
        "{rendered}"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn net_rejects_read_and_write_calls_over_the_per_call_limit(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("TCP listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("TCP connection");
        stream
            .set_read_timeout(Some(std::time::Duration::from_millis(250)))
            .expect("timeout");
        let mut stream = stream;
        let mut byte = [0];
        let _ = stream.read(&mut byte);
    });

    let source = NET_LIMIT_PROBE.replace("__PORT__", &port.to_string());
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("socket write exceeded the 1048576 byte limit")
            && rendered.contains("socket read exceeded the 1048576 byte limit"),
        "{rendered}"
    );
    server.join().expect("TCP server");
}

#[gpui::test]
fn two_runtimes_keep_distinct_network_policies(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one allowed connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
            .expect("HTTP response");
    });
    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/policy"));

    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().network_hosts(["127.0.0.1".to_owned()]));
    let allowed_runtime = ShellRuntime::new_isolated().expect("allowed runtime");
    allowed_runtime.use_direct_http_for_tests();
    cx.update(|cx| allowed_runtime.set_global(cx));
    let allowed_type = allowed_runtime
        .load_source("allowed-network.js", &source)
        .expect("allowed source");
    let allowed_window = cx.add_window(|_, _| Empty);
    let mut allowed_context = VisualTestContext::from_window(*allowed_window.deref(), cx);
    let allowed_view = allowed_context
        .update(|window, cx| allowed_runtime.instantiate_view(&allowed_type, window, cx))
        .expect("allowed view");

    crate::set_capabilities(Capabilities::new());
    let denied_runtime = ShellRuntime::new_isolated().expect("denied runtime");
    cx.update(|cx| denied_runtime.set_global(cx));
    let denied_type = denied_runtime
        .load_source("denied-network.js", &source)
        .expect("denied source");
    let denied_window = cx.add_window(|_, _| Empty);
    let mut denied_context = VisualTestContext::from_window(*denied_window.deref(), cx);
    let denied_view = denied_context
        .update(|window, cx| denied_runtime.instantiate_view(&denied_type, window, cx))
        .expect("denied view");

    allowed_context.run_until_parked();
    denied_context.run_until_parked();
    draw(&mut allowed_context, &allowed_view);
    draw(&mut denied_context, &denied_view);
    assert!(
        snapshot(&mut allowed_context, &allowed_view).contains("200|true|hello"),
        "allowed runtime lost its captured policy"
    );
    let denied = snapshot(&mut denied_context, &denied_view);
    assert!(
        denied.contains("rejected:") && denied.contains("capabilities.network.hosts"),
        "{denied}"
    );
    server.join().expect("HTTP server");
}

fn probe(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().network_hosts(["127.0.0.1".to_owned()]));
    let runtime = ShellRuntime::new_isolated().expect("runtime");
    runtime.use_direct_http_for_tests();
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("network.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    (runtime, view, context)
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

fn snapshot(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}
