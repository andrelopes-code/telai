use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use telai_stream::protocol::Status;
use telai_stream::{Asset, AssetSource, Hub, Server};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn root_serves_the_player_page() {
    let (port, _hub) = start().await;

    let response = http_get(port, "/", &format!("127.0.0.1:{port}")).await;

    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("content-type: text/html"), "{response}");
    assert!(
        response.contains("content-security-policy: default-src 'self'"),
        "{response}"
    );
    assert!(response.ends_with("<h1>player</h1>"), "{response}");
}

#[tokio::test]
async fn paths_outside_the_player_are_not_served() {
    let (port, _hub) = start().await;

    let response = http_get(port, "/index.html", &format!("127.0.0.1:{port}")).await;

    assert!(response.starts_with("HTTP/1.1 404"), "{response}");
}

#[tokio::test]
async fn a_foreign_host_header_is_rejected() {
    let (port, _hub) = start().await;

    let response = http_get(port, "/", "attacker.example").await;

    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
}

#[tokio::test]
async fn a_viewer_receives_status_then_the_latest_frame() {
    let (port, hub) = start().await;
    hub.set_status(Status {
        live: true,
        title: "Demo".into(),
    });
    hub.publish_video(Bytes::from_static(b"frame"));

    let mut socket = connect(port, &format!("http://127.0.0.1:{port}"))
        .await
        .unwrap();

    let status = next_message(&mut socket).await;
    assert_eq!(
        status,
        Message::Text(r#"{"live":true,"title":"Demo"}"#.into())
    );

    let frame = next_message(&mut socket).await;
    assert_eq!(frame, Message::Binary(Bytes::from_static(b"frame")));
}

#[tokio::test]
async fn a_viewer_receives_audio_published_after_it_connected() {
    let (port, hub) = start().await;
    let mut socket = connect(port, &format!("http://localhost:{port}"))
        .await
        .unwrap();
    next_message(&mut socket).await;
    wait_for_viewers(&hub, 1).await;

    hub.publish_audio(&[0.25, -0.25]);

    let Message::Binary(packet) = next_message(&mut socket).await else {
        panic!("expected a binary audio packet");
    };
    assert_eq!(packet[0], 2);
    assert_eq!(&packet[4..8], &0.25_f32.to_le_bytes());
}

#[tokio::test]
async fn a_foreign_origin_cannot_open_the_stream() {
    let (port, _hub) = start().await;

    let result = connect(port, "https://attacker.example").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn disconnected_viewers_are_removed() {
    let (port, hub) = start().await;
    let socket = connect(port, &format!("http://127.0.0.1:{port}"))
        .await
        .unwrap();
    wait_for_viewers(&hub, 1).await;

    drop(socket);

    wait_for_viewers(&hub, 0).await;
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;

async fn start() -> (u16, Hub) {
    let server = Server::bind(0).await.unwrap();
    let port = server.port();
    let hub = Hub::new();

    let assets: AssetSource = Arc::new(|path| {
        (path == "player.html").then(|| Asset {
            body: b"<h1>player</h1>".to_vec(),
            mime_type: "text/html".into(),
        })
    });
    tokio::spawn(server.serve(hub.clone(), assets));

    (port, hub)
}

async fn connect(port: u16, origin: &str) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
    let mut request = format!("ws://127.0.0.1:{port}/ws")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("Origin", HeaderValue::from_str(origin).unwrap());

    let (socket, _) = tokio_tungstenite::connect_async(request).await?;

    Ok(socket)
}

async fn next_message(socket: &mut Socket) -> Message {
    tokio::time::timeout(TIMEOUT, socket.next())
        .await
        .expect("message before timeout")
        .expect("open socket")
        .expect("valid message")
}

async fn wait_for_viewers(hub: &Hub, count: usize) {
    let mut viewers = hub.viewers();

    tokio::time::timeout(TIMEOUT, viewers.wait_for(|viewers| viewers.len() == count))
        .await
        .expect("viewer count before timeout")
        .unwrap();
}

async fn http_get(port: u16, path: &str, host: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();

    response
}
