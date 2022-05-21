use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::user::UserData;

mod dashboard;

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
	user: &UserData,
) {
	dashboard::run_dashboard(ws_write, ws_read, user).await;
}
