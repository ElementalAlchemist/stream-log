use crate::pages::error::ErrorData;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::RequestMessage;

/// Sends an unsubscribe all message to the web server
/// TODO: Handle race conditions by switching communication paradigms again
pub async fn send_unsubscribe_all_message(ws: &mut WebSocket) -> Result<(), ErrorData> {
	let message = RequestMessage::UnsubscribeAll;
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			return Err(ErrorData::new_with_error(
				"Failed to serialize unsubscribe request message",
				error,
			))
		}
	};
	if let Err(error) = ws.send(Message::Text(message_json)).await {
		return Err(ErrorData::new_with_error(
			"Failed to send unsubscribe request message",
			error,
		));
	}
	Ok(())
}
