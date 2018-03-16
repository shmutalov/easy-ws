extern crate ws;

use std::borrow::Cow;
use std::thread;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::time::Duration;

#[derive(Debug)]
pub enum EasyWsError {
    Unknown,
}

impl From<SendError<EasyWsCommand>> for EasyWsError {
    fn from(_e: SendError<EasyWsCommand>) -> Self {
        EasyWsError::Unknown
    }
}

pub enum EasyWsCommand {
    Disconnect,
    Send(String),
}

#[derive(PartialEq)]
pub enum EasyWsConnectionState {
    Connecting,
    Handshake,
    Connected,
    Disconnected,
}

pub type EasyWsResult = Result<(), EasyWsError>;

struct WsClient {
    out: ws::Sender,
}

impl ws::Handler for WsClient {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        Ok(())
    }
}

pub struct SimpleWebSocket {
    _rx: Receiver<EasyWsCommand>,
    _tx: Sender<EasyWsCommand>,

    timeout: Duration,
    interval: Duration,
    endpoint: String,

    on_connect_fn: Option<Box<FnMut() + Send>>,
    on_disconnect_fn: Option<Box<FnMut() + Send>>,
    on_message_fn: Option<Box<FnMut(&str) + Send>>,
    on_error_fn: Option<Box<FnMut(&str) + Send>>,

    state: EasyWsConnectionState,
}

impl SimpleWebSocket {
    pub fn new<S>(endpoint: S, timeout_ms: u64, interval: u64) -> SimpleWebSocket
    where
        S: AsRef<str>,
    {
        let (tx, rx) = channel();

        SimpleWebSocket {
            _rx: rx,
            _tx: tx,
            timeout: Duration::from_millis(timeout_ms),
            interval: Duration::from_millis(interval),
            endpoint: endpoint.as_ref().to_string(),
            on_connect_fn: None,
            on_disconnect_fn: None,
            on_message_fn: None,
            on_error_fn: None,
            state: EasyWsConnectionState::Disconnected,
        }
    }

    pub fn connect(&mut self) -> EasyWsResult {
        if self.state != EasyWsConnectionState::Disconnected {
            ()
        }

        let endpoint = self.endpoint.clone();

        thread::spawn(|| {
            // start websocket event-loop
            if let Err(error) = ws::connect(endpoint, |out| WsClient { out: out }) {
                if let Some(func) = self.on_error_fn {
                    (func)(error.description());
                }
            }
        });

        Ok(())
    }

    pub fn disconnect(&mut self) -> EasyWsResult {
        if self.state == EasyWsConnectionState::Disconnected {
            ()
        }

        self._tx.send(EasyWsCommand::Disconnect)?;

        Ok(())
    }

    pub fn send<S>(&mut self, message: S) -> EasyWsResult
    where
        S: AsRef<str>,
    {
        let msg = message.as_ref().to_string();
        self._tx.send(EasyWsCommand::Send(msg))?;

        Ok(())
    }

    pub fn on_connect<F>(&mut self, callback: F)
    where
        F: 'static + FnMut() + Send,
    {
        self.on_connect_fn = Some(Box::new(callback));
    }

    pub fn on_disconnect<F>(&mut self, callback: F)
    where
        F: 'static + FnMut() + Send,
    {
        self.on_disconnect_fn = Some(Box::new(callback));
    }

    pub fn on_message<F>(&mut self, callback: F)
    where
        F: 'static + FnMut(&str) + Send,
    {
        self.on_message_fn = Some(Box::new(callback));
    }

    pub fn on_error<F>(&mut self, callback: F)
    where
        F: 'static + FnMut(&str) + Send,
    {
        self.on_error_fn = Some(Box::new(callback));
    }
}

pub struct SimpleWebSocketBuilder<'a> {
    timeout: u64,
    interval: u64,
    endpoint: Cow<'a, str>,
}

impl<'a> SimpleWebSocketBuilder<'a> {
    pub fn new<S>(endpoint: S) -> SimpleWebSocketBuilder<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        SimpleWebSocketBuilder {
            timeout: 10000,
            interval: 1000,
            endpoint: endpoint.into(),
        }
    }

    pub fn with_timeout(&mut self, milliseconds: u64) -> &mut SimpleWebSocketBuilder<'a> {
        self.timeout = milliseconds;
        self
    }

    pub fn with_interval(&mut self, milliseconds: u64) -> &mut SimpleWebSocketBuilder<'a> {
        self.interval = milliseconds;
        self
    }

    pub fn build(&self) -> SimpleWebSocket {
        SimpleWebSocket::new(&self.endpoint, self.timeout, self.interval)
    }
}
