use super::error::{ErrorData, ErrorView};
use crate::components::event_log_entry::{EventLogEntryEdit, EventLogEntryRow};
use crate::subscriptions::send_unsubscribe_all_message;
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::event_subscription::{EventSubscriptionResponse, EventSubscriptionUpdate};
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::RequestMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;

#[derive(Prop)]
pub struct EventLogProps {
	id: String,
}

#[derive(Clone, Eq, Hash, PartialEq)]
enum ModifiedEventLogEntryParts {
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLink,
	SubmitterOrWinner,
	Tags,
	MakeVideo,
	NotesToEditor,
	Editor,
	Highlighted,
}

#[component]
async fn EventLogLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	if let Err(error) = send_unsubscribe_all_message(&mut ws).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(error));
		return view! { ctx, ErrorView };
	}

	let subscribe_msg = RequestMessage::SubscribeToEvent(props.id.clone());
	let subscribe_msg_json = match serde_json::to_string(&subscribe_msg) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize event subscription request message",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	if let Err(error) = ws.send(Message::Text(subscribe_msg_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send event subscription request message",
			error,
		)));
		return view! { ctx, ErrorView };
	}

	let subscribe_response: EventSubscriptionResponse = match read_websocket(&mut ws).await {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive event subscription response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let (event, permission_level, entry_types, tags, event_editors, log_entries) = match subscribe_response {
		EventSubscriptionResponse::Subscribed(
			event,
			permission_level,
			event_types,
			tags,
			event_editors,
			log_entries,
		) => (event, permission_level, event_types, tags, event_editors, log_entries),
		EventSubscriptionResponse::NoEvent => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new("That event does not exist")));
			return view! { ctx, ErrorView };
		}
		EventSubscriptionResponse::NotAllowed => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new("Not allowed to access that event")));
			return view! { ctx, ErrorView };
		}
		EventSubscriptionResponse::Error(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"An error occurred subscribing to event updates",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_signal = create_signal(ctx, event);
	let permission_signal = create_signal(ctx, permission_level);
	let entry_types_signal = create_signal(ctx, entry_types);
	let tags_signal = create_signal(ctx, tags);
	let log_entries = create_signal(ctx, log_entries);
	let available_editors = create_signal(ctx, event_editors);

	let tags_by_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Tag> = tags_signal
			.get()
			.iter()
			.map(|tag| (tag.name.clone(), tag.clone()))
			.collect();
		name_index
	});
	let editors_by_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, UserData> = available_editors
			.get()
			.iter()
			.map(|editor| (editor.username.clone(), editor.clone()))
			.collect();
		name_index
	});
	let can_edit = create_memo(ctx, || *permission_signal.get() == PermissionLevel::Edit);

	view! {
		ctx,
		h1(id="stream_log_event_title") { (event_signal.get().name) }
		div(id="event_log") {
			Keyed(
				iterable=log_entries,
				key=|entry| entry.id.clone(),
				view=move |ctx, entry| {
					let entry_types = entry_types_signal.get();
					let entry_type = (*entry_types).iter().find(|et| et.id == entry.entry_type).unwrap();
					let event = event_signal.get();
					let edit_open_signal = create_signal(ctx, false);
					let click_handler = if *can_edit.get() {
						Some(|| { edit_open_signal.set(true); })
					} else {
						None
					};
					let event_log_entry_signal = create_signal(ctx, Some(entry.clone()));

					// Set up edit signals/data
					let edit_start_time = create_signal(ctx, entry.start_time);
					let edit_end_time = create_signal(ctx, entry.end_time);
					let edit_entry_type = create_signal(ctx, entry.entry_type.clone());
					let edit_description = create_signal(ctx, entry.description.clone());
					let edit_media_link = create_signal(ctx, entry.media_link.clone());
					let edit_submitter_or_winner = create_signal(ctx, entry.submitter_or_winner.clone());
					let edit_tags = create_signal(ctx, entry.tags.clone());
					let edit_make_video = create_signal(ctx, entry.make_video);
					let edit_notes_to_editor = create_signal(ctx, entry.notes_to_editor.clone());
					let edit_editor = create_signal(ctx, entry.editor.clone());
					let edit_highlighted = create_signal(ctx, entry.highlighted);

					let modified_data: &Signal<HashSet<ModifiedEventLogEntryParts>> = create_signal(ctx, HashSet::new());
					let ran_once: &Signal<HashSet<ModifiedEventLogEntryParts>> = create_signal(ctx, HashSet::new());

					create_effect(ctx, || {
						edit_start_time.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::StartTime) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::StartTime);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::StartTime);
					});
					create_effect(ctx, || {
						edit_end_time.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::EndTime) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::EndTime);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
					});
					create_effect(ctx, || {
						edit_entry_type.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::EntryType) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::EntryType);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::EntryType);
					});
					create_effect(ctx, || {
						edit_description.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::Description) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::Description);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::Description);
					});
					create_effect(ctx, || {
						edit_media_link.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::MediaLink) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::MediaLink);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::MediaLink);
					});
					create_effect(ctx, || {
						edit_submitter_or_winner.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::SubmitterOrWinner) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::SubmitterOrWinner);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::SubmitterOrWinner);
					});
					create_effect(ctx, || {
						edit_tags.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::Tags) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::Tags);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::Tags);
					});
					create_effect(ctx, || {
						edit_make_video.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::MakeVideo) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::MakeVideo);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::MakeVideo);
					});
					create_effect(ctx, || {
						edit_notes_to_editor.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::NotesToEditor) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::NotesToEditor);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::NotesToEditor);
					});
					create_effect(ctx, || {
						edit_editor.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::Editor) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::Editor);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::Editor);
					});
					create_effect(ctx, || {
						edit_highlighted.track();
						if !ran_once.get_untracked().contains(&ModifiedEventLogEntryParts::Highlighted) {
							ran_once.modify().insert(ModifiedEventLogEntryParts::Highlighted);
							return;
						}
						modified_data.modify().insert(ModifiedEventLogEntryParts::Highlighted);
					});

					let close_handler_entry = entry.clone();

					view! {
						ctx,
						EventLogEntryRow(entry=entry, event=(*event).clone(), entry_type=entry_type.clone(), click_handler=click_handler)
						(if *edit_open_signal.get() {
							let close_handler = {
								let entry = close_handler_entry.clone();
								move || {
									let entry = entry.clone();
									spawn_local_scoped(ctx, async move {
										edit_open_signal.set(false);

										let mut log_entries = log_entries.modify();
										let log_entry = log_entries.iter_mut().find(|log_entry| log_entry.id == entry.id);
										let log_entry = match log_entry {
											Some(entry) => entry,
											None => return
										};

										let event = (*event_signal.get()).clone();

										let ws_context: &Mutex<WebSocket> = use_context(ctx);
										let mut ws = ws_context.lock().await;

										let modified_data = modified_data.modify();
										for changed_datum in modified_data.iter() {
											let event_message = match changed_datum {
												ModifiedEventLogEntryParts::StartTime => EventSubscriptionUpdate::ChangeStartTime(log_entry.clone(), *edit_start_time.get()),
												ModifiedEventLogEntryParts::EndTime => EventSubscriptionUpdate::ChangeEndTime(log_entry.clone(), *edit_end_time.get()),
												ModifiedEventLogEntryParts::EntryType => EventSubscriptionUpdate::ChangeEntryType(log_entry.clone(), (*edit_entry_type.get()).clone()),
												ModifiedEventLogEntryParts::Description => EventSubscriptionUpdate::ChangeDescription(log_entry.clone(), (*edit_description.get()).clone()),
												ModifiedEventLogEntryParts::MediaLink => EventSubscriptionUpdate::ChangeMediaLink(log_entry.clone(), (*edit_media_link.get()).clone()),
												ModifiedEventLogEntryParts::SubmitterOrWinner => EventSubscriptionUpdate::ChangeSubmitterWinner(log_entry.clone(), (*edit_submitter_or_winner.get()).clone()),
												ModifiedEventLogEntryParts::Tags => EventSubscriptionUpdate::ChangeTags(log_entry.clone(), (*edit_tags.get()).clone()),
												ModifiedEventLogEntryParts::MakeVideo => EventSubscriptionUpdate::ChangeMakeVideo(log_entry.clone(), *edit_make_video.get()),
												ModifiedEventLogEntryParts::NotesToEditor => EventSubscriptionUpdate::ChangeNotesToEditor(log_entry.clone(), (*edit_notes_to_editor.get()).clone()),
												ModifiedEventLogEntryParts::Editor => EventSubscriptionUpdate::ChangeEditor(log_entry.clone(), (*edit_editor.get()).clone()),
												ModifiedEventLogEntryParts::Highlighted => EventSubscriptionUpdate::ChangeHighlighted(log_entry.clone(), *edit_highlighted.get())
											};
											let event_message = RequestMessage::EventSubscriptionUpdate(event.clone(), Box::new(event_message));
											let event_message = match serde_json::to_string(&event_message) {
												Ok(msg) => msg,
												Err(error) => {
													let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
													error_signal.set(Some(ErrorData::new_with_error("Failed to serialize outgoing entry log change", error)));
													navigate("/error");
													return;
												}
											};
											if let Err(error) = ws.send(Message::Text(event_message)).await {
												let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
												error_signal.set(Some(ErrorData::new_with_error("Failed to send outgoing log entry change", error)));
												navigate("/error");
											}
										}
									});
								}
							};
							view! {
								ctx,
								EventLogEntryEdit(
									event=event_signal,
									event_entry_types=entry_types_signal,
									event_tags_name_index=tags_by_name_index,
									entry_types_datalist_id="event_entry_types",
									event_log_entry=event_log_entry_signal,
									start_time=edit_start_time,
									end_time=edit_end_time,
									entry_type=edit_entry_type,
									description=edit_description,
									media_link=edit_media_link,
									submitter_or_winner=edit_submitter_or_winner,
									tags=edit_tags,
									make_video=edit_make_video,
									notes_to_editor=edit_notes_to_editor,
									editor=edit_editor,
									editor_name_index=editors_by_name_index,
									editor_name_datalist_id="editor_names",
									highlighted=edit_highlighted,
									close_handler=close_handler,
									editing_new=false
								)
							}
						} else {
							view! { ctx, }
						})
					}
				}
			)
		}
		datalist(id="event_entry_types") {
			Keyed(
				iterable=entry_types_signal,
				key=|entry_type| entry_type.id.clone(),
				view=|ctx, entry_type| {
					let type_name = entry_type.name;
					view! {
						ctx,
						option(value=type_name)
					}
				}
			)
		}
		datalist(id="editor_names") {
			Keyed(
				iterable=available_editors,
				key=|editor| editor.id.clone(),
				view=|ctx, editor| {
					let editor_name = editor.username;
					view! {
						ctx,
						option(value=editor_name)
					}
				}
			)
		}
	}
}

#[component]
pub fn EventLogView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event log data..." }) {
			EventLogLoadedView(id=props.id)
		}
	}
}
