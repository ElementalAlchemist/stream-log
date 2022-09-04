use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::user::UserData;

pub mod menu;

pub async fn run_admin_page(user: &UserData, ws: &mut WebSocket) {
	menu::handle_menu_page(user, ws).await;
}
