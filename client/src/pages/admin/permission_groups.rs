use crate::error::PageError;
use crate::websocket::read_websocket;
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminAction, EventPermission, PermissionGroup, PermissionGroupWithEvents};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::{DataMessage, SubPageControl};

enum PageAction {
	Rename(PermissionGroup),
	AddEvent(PermissionGroup, EventPermission),
	RemoveEvent(PermissionGroup, EventPermission),
	AddGroup(String),
	ReturnFromPage,
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), PageError> {
	loop {
		let list_request = SubPageControl::Event(AdminAction::ListPermissionGroups);
		let list_request_json = serde_json::to_string(&list_request)?;
		ws_write.send(Message::Text(list_request_json)).await?;
		let event_request = SubPageControl::Event(AdminAction::ListEvents);
		let response: DataMessage<Vec<PermissionGroupWithEvents>> = read_websocket(ws_read).await?;
		let event_response: DataMessage<Vec<Event>> = read_websocket(ws_read).await?;
		let response = response?;
		let mut event_response = event_response?;
		let events: HashMap<String, Event> = event_response
			.drain(..)
			.map(|event| (event.id.clone(), event))
			.collect();
		let (action_tx, action_rx) = mpsc::channel(1);
		let permission_list_items: Vec<ViewBuilder<Dom>> = response
			.iter()
			.map(|group| list_item_view(group.clone(), &action_tx))
			.collect();
	}
}

fn list_item_view(group_data: PermissionGroupWithEvents, action_tx: &mpsc::Sender<PageAction>) -> ViewBuilder<Dom> {
	let event_list: Vec<ViewBuilder<Dom>> = group_data
		.events
		.iter()
		.map(|event| {
			let event = event.clone();
			builder! {
				<li
					data_event_id=event.event.id.clone()
				>
					<span class="admin-permission-group-event-name">
						{event.event.name.clone()}
					</span>
					<span class="admin-permission-group-event-level">
						{
							match event.level {
								PermissionLevel::View => "View",
								PermissionLevel::Edit => "Edit"
							}
						}
					</span>
					<button
						class="admin-permission-group-event-remove"
						on:click=action_tx.sink().contra_map({
							let group = group_data.group.clone();
							move |_: DomEvent| PageAction::RemoveEvent(group.clone(), event.clone())
						})
					>
						"Remove Event"
					</button>
				</li>
			}
		})
		.collect();
	builder! {
		<li
			class="admin-permission-group-data"
			data_group_id=group_data.group.id.clone()
		>
			<div class="admin-permission-group-name">
				{group_data.group.name.clone()}
			</div>
			<form
				class="admin-permission-group-rename"
				on:submit=action_tx.sink().contra_map({
					let group = group_data.group.clone();
					move |event: DomEvent| {
						event.browser_event().unwrap().prevent_default();
						PageAction::Rename(group.clone())
					}
				})
			>
				<input
					type="text"
					name="group_name"
				/>
				<button type="submit">"Rename"</button>
			</form>
			<ul
				class="admin-permission-group-events"
			>
				{event_list}
			</ul>
			<form
				class="admin-permission-group-events-add"
				on:submit=action_tx.sink().contra_map(|event: DomEvent| {
					let event = event.browser_event().unwrap();
					event.prevent_default();
					todo!()
				})
			>
			</form>
		</li>
	}
}
