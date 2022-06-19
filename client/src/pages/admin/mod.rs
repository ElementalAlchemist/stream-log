use crate::error::PageError;
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;

mod events;
mod menu;
mod permission_groups;
mod users;

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), PageError> {
	menu::run_menu(ws_write, ws_read).await
}
