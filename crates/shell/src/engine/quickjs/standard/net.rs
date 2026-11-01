use crate::engine::quickjs::js_error;
use std::{
    io::{Read as _, Write as _},
    net::{Shutdown, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

use rquickjs::{
    Ctx, IntoJs, Object, Promise, Result, TypedArray, Value,
    function::{Func, Opt},
    module::{Declarations, Exports, ModuleDef},
};

use super::{
    super::{host, scheduler},
    connect as network_connect,
};

const IO_LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("connect")?;
        declarations.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let net = Object::new(ctx.clone())?;
        net.set("connect", Func::from(connect))?;
        exports.export("connect", net.get::<_, Value>("connect")?)?;
        exports.export("default", net.into_value())?;
        Ok(())
    }
}

fn connect<'js>(ctx: Ctx<'js>, host_name: String, port: u16) -> Result<Promise<'js>> {
    let normalized = host_name.to_ascii_lowercase();
    if !host::capabilities().may_reach(&normalized) {
        return Err(js_error::type_error(
            &ctx,
            &format!(
                "network access to `{normalized}` is not granted; add it to capabilities.network.hosts"
            ),
        ));
    }
    scheduler::blocking(&ctx, "net.connect(host, port)", move || {
        let operation = format!("{host_name}:{port}");
        let (stream, _) = network_connect::connect_tcp(&host_name, port, TIMEOUT, &operation)?;
        stream.set_read_timeout(Some(TIMEOUT)).ok();
        stream.set_write_timeout(Some(TIMEOUT)).ok();
        let reader = stream
            .try_clone()
            .map_err(|error| format!("cloning {host_name}:{port} for reads failed: {error}"))?;
        let writer = stream
            .try_clone()
            .map_err(|error| format!("cloning {host_name}:{port} for writes failed: {error}"))?;
        Ok(Socket {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            closer: stream,
        })
    })
}

struct Socket {
    reader: Arc<Mutex<TcpStream>>,
    writer: Arc<Mutex<TcpStream>>,
    closer: TcpStream,
}

struct ReadChunk(Option<Vec<u8>>);

impl<'js> IntoJs<'js> for ReadChunk {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self.0 {
            Some(bytes) => TypedArray::<u8>::new(ctx.clone(), bytes)?.into_js(ctx),
            None => Ok(Value::new_null(ctx.clone())),
        }
    }
}

impl<'js> IntoJs<'js> for Socket {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let socket = Object::new(ctx.clone())?;
        let writer = self.writer;
        socket.set(
            "write",
            Func::from(move |ctx: Ctx<'js>, data: String| -> Result<Promise<'js>> {
                if data.len() > IO_LIMIT {
                    return Err(js_error::range_error(
                        &ctx,
                        &format!("socket write exceeded the {IO_LIMIT} byte limit"),
                    ));
                }
                let stream = writer.clone();
                scheduler::blocking(&ctx, "socket.write(data)", move || {
                    stream
                        .lock()
                        .map_err(|_| "socket lock was poisoned".to_owned())?
                        .write_all(data.as_bytes())
                        .map_err(|error| format!("socket write failed: {error}"))
                })
            }),
        )?;
        let reader = self.reader;
        socket.set(
            "read",
            Func::from(
                move |ctx: Ctx<'js>, size: Opt<usize>| -> Result<Promise<'js>> {
                    let size = size.0.unwrap_or(64 * 1024);
                    if size > IO_LIMIT {
                        return Err(js_error::range_error(
                            &ctx,
                            &format!("socket read exceeded the {IO_LIMIT} byte limit"),
                        ));
                    }
                    let stream = reader.clone();
                    scheduler::blocking(&ctx, "socket.read(size)", move || {
                        let mut bytes = vec![0; size];
                        let count = stream
                            .lock()
                            .map_err(|_| "socket lock was poisoned".to_owned())?
                            .read(&mut bytes)
                            .map_err(|error| format!("socket read failed: {error}"))?;
                        if count == 0 {
                            return Ok(ReadChunk(None));
                        }
                        bytes.truncate(count);
                        Ok(ReadChunk(Some(bytes)))
                    })
                },
            ),
        )?;
        let closer = self.closer;
        socket.set(
            "close",
            Func::from(move || {
                let _ = closer.shutdown(Shutdown::Both);
            }),
        )?;
        Ok(socket.into_value())
    }
}
