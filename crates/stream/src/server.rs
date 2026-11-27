use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::sync::broadcast::error::RecvError;

use crate::hub::{Hub, Subscription};
use crate::protocol::Status;

/// Fixed so that the page URL, and the per-site mute the browser stores for
/// it, stay the same across sessions.
pub const DEFAULT_PORT: u16 = 47823;

const PLAYER_PAGE: &str = "player.html";

/// The page loads only its own files and talks only to this server.
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; img-src 'self' data:";
const ASSETS_PREFIX: &str = "assets/";

/// A file of the player page.
pub struct Asset {
    pub body: Vec<u8>,
    pub mime_type: String,
}

/// Looks up a player file by its path relative to the build output.
pub type AssetSource = Arc<dyn Fn(&str) -> Option<Asset> + Send + Sync>;

/// HTTP and WebSocket server for the player page, bound to the loopback
/// interface only.
pub struct Server {
    listener: TcpListener,
    port: u16,
}

#[derive(Clone)]
struct AppState {
    hub: Hub,
    assets: AssetSource,
    allowed_hosts: Arc<[String]>,
}

impl Server {
    pub async fn bind(port: u16) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
        let port = listener.local_addr()?.port();

        Ok(Self { listener, port })
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://{}:{}", Ipv4Addr::LOCALHOST, self.port)
    }

    /// Runs until the listener fails.
    pub async fn serve(self, hub: Hub, assets: AssetSource) -> io::Result<()> {
        let state = AppState {
            hub,
            assets,
            allowed_hosts: Arc::from([
                format!("{}:{}", Ipv4Addr::LOCALHOST, self.port),
                format!("localhost:{}", self.port),
            ]),
        };

        let router = Router::new()
            .route("/ws", get(open_websocket))
            .fallback(get(serve_asset))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                reject_foreign_requests,
            ))
            .with_state(state);

        axum::serve(
            self.listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    }
}

/// Blocks DNS rebinding (a foreign `Host`) and other sites opening the
/// stream (a foreign `Origin`). Browsers send `Origin` on every WebSocket
/// handshake, so a missing one is rejected as well.
async fn reject_foreign_requests(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    let host_allowed = header_matches(headers, header::HOST, &state.allowed_hosts, "");

    let is_websocket = request.uri().path() == "/ws";
    let origin_allowed =
        !is_websocket || header_matches(headers, header::ORIGIN, &state.allowed_hosts, "http://");

    if !host_allowed || !origin_allowed {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(request).await
}

fn header_matches(
    headers: &HeaderMap,
    name: header::HeaderName,
    allowed_hosts: &[String],
    scheme: &str,
) -> bool {
    let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) else {
        return false;
    };

    allowed_hosts
        .iter()
        .any(|host| value.strip_prefix(scheme) == Some(host.as_str()))
}

async fn serve_asset(State(state): State<AppState>, uri: Uri) -> Response {
    let path = match uri.path().trim_start_matches('/') {
        "" => PLAYER_PAGE,
        path if path.starts_with(ASSETS_PREFIX) => path,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    let Some(asset) = (state.assets)(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = HeaderValue::from_str(&asset.mime_type)
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));

    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(CONTENT_SECURITY_POLICY),
            ),
        ],
        asset.body,
    )
        .into_response()
}

async fn open_websocket(
    upgrade: WebSocketUpgrade,
    ConnectInfo(address): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Response {
    upgrade.on_upgrade(move |socket| stream_to_viewer(socket, address, state.hub))
}

async fn stream_to_viewer(mut socket: WebSocket, address: SocketAddr, hub: Hub) {
    hub.add_viewer(address);

    if let Err(error) = forward(&mut socket, hub.subscribe()).await {
        tracing::debug!(%address, %error, "viewer connection ended");
    }

    hub.remove_viewer(address);
}

/// Sends media to one viewer until either side closes.
async fn forward(
    socket: &mut WebSocket,
    mut subscription: Subscription,
) -> Result<(), axum::Error> {
    let status = subscription.status.borrow_and_update().clone();
    socket.send(status_message(&status)).await?;

    loop {
        tokio::select! {
            changed = subscription.video.changed() => {
                if changed.is_err() {
                    return Ok(());
                }

                let frame = subscription.video.borrow_and_update().clone();
                if let Some(frame) = frame {
                    socket.send(Message::Binary(frame)).await?;
                }
            }
            packet = subscription.audio.recv() => match packet {
                Ok(packet) => socket.send(Message::Binary(packet)).await?,
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return Ok(()),
            },
            changed = subscription.status.changed() => {
                if changed.is_err() {
                    return Ok(());
                }

                let status = subscription.status.borrow_and_update().clone();
                socket.send(status_message(&status)).await?;
            }
            message = socket.recv() => match message {
                None | Some(Ok(Message::Close(_))) => return Ok(()),
                Some(Err(error)) => return Err(error),
                Some(Ok(_)) => {}
            },
        }
    }
}

fn status_message(status: &Status) -> Message {
    let json = serde_json::to_string(status).expect("status serializes to JSON");

    Message::Text(json.into())
}
