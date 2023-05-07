use crate::components::event_log_entry::{EventLogEntryEdit, EventLogEntryRow};
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use chrono::{DateTime, Utc};
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::task::{Context, Poll, Waker};
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionUpdate;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

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
	log::debug!("Starting event log load for event {}", props.id);

	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	log::debug!("Got websocket to load event {}", props.id);

	let data: &DataSignals = use_context(ctx);

	let add_subscription_data = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscriptions(
				vec![
					SubscriptionType::EventLogData(props.id.clone()),
					SubscriptionType::AvailableTags,
				],
				&mut ws,
			)
			.await
	};
	if let Err(error) = add_subscription_data {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}
	log::debug!("Added subscription data for event {}", props.id);

	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"Checking whether event {} is present yet in the subscription manager",
			props.id
		);
		match data.events.get().get(&props.id) {
			Some(event_subscription_data) => Poll::Ready(event_subscription_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Waker>> = use_context(ctx);
				event_wakers
					.modify()
					.insert(props.id.clone(), poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let event_signal = event_subscription_data.event.clone();
	let permission_signal = event_subscription_data.permission.clone();
	let entry_types_signal = event_subscription_data.entry_types.clone();
	let tags_signal = data.available_tags.clone();
	let log_entries = event_subscription_data.event_log_entries.clone();
	let available_editors = event_subscription_data.editors;

	let read_event_signal = create_memo(ctx, {
		let event_signal = event_signal.clone();
		move || (*event_signal.get()).clone()
	});
	let read_entry_types_signal = create_memo(ctx, {
		let entry_types_signal = entry_types_signal.clone();
		move || (*entry_types_signal.get()).clone()
	});
	let read_tags_signal = create_memo(ctx, {
		let tags_signal = tags_signal.clone();
		move || (*tags_signal.get()).clone()
	});
	let read_log_entries = create_memo(ctx, {
		let log_entries = log_entries.clone();
		move || (*log_entries.get()).clone()
	});
	let read_available_editors = create_memo(ctx, {
		let available_editors = available_editors.clone();
		move || (*available_editors.get()).clone()
	});

	let tags_by_name_index = create_memo(ctx, move || {
		let name_index: HashMap<String, Tag> = tags_signal
			.get()
			.iter()
			.map(|tag| (tag.name.clone(), tag.clone()))
			.collect();
		name_index
	});
	let editors_by_name_index = create_memo(ctx, move || {
		let name_index: HashMap<String, UserData> = available_editors
			.get()
			.iter()
			.map(|editor| (editor.username.clone(), editor.clone()))
			.collect();
		name_index
	});
	let can_edit = create_memo(ctx, move || *permission_signal.get() == PermissionLevel::Edit);

	log::debug!("Set up loaded data signals for event {}", props.id);

	let new_event_log_entry: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);
	let new_entry_start_time = create_signal(ctx, Utc::now());
	let new_entry_end_time: &Signal<Option<DateTime<Utc>>> = create_signal(ctx, None);
	let new_entry_type = create_signal(ctx, String::new());
	let new_entry_description = create_signal(ctx, String::new());
	let new_entry_media_link = create_signal(ctx, String::new());
	let new_entry_submitter_or_winner = create_signal(ctx, String::new());
	let new_entry_tags: &Signal<Vec<Tag>> = create_signal(ctx, Vec::new());
	let new_entry_make_video = create_signal(ctx, false);
	let new_entry_notes_to_editor = create_signal(ctx, String::new());
	let new_entry_editor: &Signal<Option<UserData>> = create_signal(ctx, None);
	let new_entry_highlighted = create_signal(ctx, false);

	let new_entry_close_handler = {
		let event_signal = event_signal.clone();
		move || {
			let event_signal = event_signal.clone();

			let start_time = *new_entry_start_time.get();
			let end_time = *new_entry_end_time.get();
			let entry_type = (*new_entry_type.get()).clone();
			let description = (*new_entry_description.get()).clone();
			let media_link = (*new_entry_media_link.get()).clone();
			let submitter_or_winner = (*new_entry_media_link.get()).clone();
			let tags = (*new_entry_tags.get()).clone();
			let make_video = *new_entry_make_video.get();
			let notes_to_editor = (*new_entry_notes_to_editor.get()).clone();
			let editor = (*new_entry_editor.get()).clone();
			let highlighted = *new_entry_highlighted.get();
			let new_event_log_entry = EventLogEntry {
				id: String::new(),
				start_time,
				end_time,
				entry_type,
				description,
				media_link,
				submitter_or_winner,
				tags,
				make_video,
				notes_to_editor,
				editor_link: None,
				editor,
				video_link: None,
				highlighted,
				parent: None,
			};

			spawn_local_scoped(ctx, async move {
				let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
				let mut ws = ws_context.lock().await;

				let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
					(*event_signal.get()).clone(),
					Box::new(EventSubscriptionUpdate::NewLogEntry(new_event_log_entry)),
				)));
				let message_json = match serde_json::to_string(&message) {
					Ok(msg) => msg,
					Err(error) => {
						data.errors.modify().push(ErrorData::new_with_error(
							"Failed to serialize new log entry submission.",
							error,
						));
						return;
					}
				};
				if let Err(error) = ws.send(Message::Text(message_json)).await {
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to send new log entry submission.",
						error,
					));
				}
			});
		}
	};

	let visible_event_signal = event_signal.clone();

	log::debug!("Created signals and handlers for event {}", props.id);

	view! {
		ctx,
		h1(id="stream_log_event_title") { (visible_event_signal.get().name) }
		div(id="event_log") {
			div(id="event_log_data") {
				div(class="event_log_header") { "Start" }
				div(class="event_log_header") { "End" }
				div(class="event_log_header") { "Type" }
				div(class="event_log_header") { "Description" }
				div(class="event_log_header") { "Submitter/Winner" }
				div(class="event_log_header") { "Media link" }
				div(class="event_log_header") { "Tags" }
				div(class="event_log_header") { }
				div(class="event_log_header") { }
				div(class="event_log_header") { }
				div(class="event_log_header") { "Editor" }
				div(class="event_log_header") { "Notes to editor" }
				Keyed(
					iterable=read_log_entries,
					key=|entry| entry.id.clone(),
					view={
						let event_signal = event_signal.clone();
						let entry_types_signal = entry_types_signal.clone();
						let log_entries = log_entries.clone();
						move |ctx, entry| {
							let event_signal = event_signal.clone();
							let entry_types_signal = entry_types_signal.clone();
							let log_entries = log_entries.clone();

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
										let event_signal = event_signal.clone();
										let log_entries = log_entries.clone();
										move || {
											let entry = entry.clone();
											let event_signal = event_signal.clone();
											let log_entries = log_entries.clone();
											spawn_local_scoped(ctx, async move {
												edit_open_signal.set(false);

												let mut log_entries = log_entries.modify();
												let log_entry = log_entries.iter_mut().find(|log_entry| log_entry.id == entry.id);
												let log_entry = match log_entry {
													Some(entry) => entry,
													None => return
												};

												let event = (*event_signal.get()).clone();

												let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
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
													let event_message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(event.clone(), Box::new(event_message))));
													let event_message = match serde_json::to_string(&event_message) {
														Ok(msg) => msg,
														Err(error) => {
															let data: &DataSignals = use_context(ctx);
															data.errors.modify().push(ErrorData::new_with_error("Failed to serialize entry log change.", error));
															return;
														}
													};
													if let Err(error) = ws.send(Message::Text(event_message)).await {
														let data: &DataSignals = use_context(ctx);
														data.errors.modify().push(ErrorData::new_with_error("Failed to send log entry change.", error));
													}
												}
											});
										}
									};
									view! {
										ctx,
										EventLogEntryEdit(
											event=read_event_signal,
											event_entry_types=read_entry_types_signal,
											event_tags_name_index=tags_by_name_index,
											entry_types_datalist_id="event_entry_types",
											event_log_entry=event_log_entry_signal,
											tags_datalist_id="event_tags",
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
					}
				)
			}
			(if *can_edit.get() {
				let new_entry_close_handler = new_entry_close_handler.clone();
				view! {
					ctx,
					div(id="event_log_new_entry") {
						EventLogEntryEdit(
							event=read_event_signal,
							event_entry_types=read_entry_types_signal,
							event_tags_name_index=tags_by_name_index,
							entry_types_datalist_id="event_entry_types",
							event_log_entry=new_event_log_entry,
							tags_datalist_id="event_tags",
							start_time=new_entry_start_time,
							end_time=new_entry_end_time,
							entry_type=new_entry_type,
							description=new_entry_description,
							media_link=new_entry_media_link,
							submitter_or_winner=new_entry_submitter_or_winner,
							tags=new_entry_tags,
							make_video=new_entry_make_video,
							notes_to_editor=new_entry_notes_to_editor,
							editor=new_entry_editor,
							editor_name_index=editors_by_name_index,
							editor_name_datalist_id="editor_names",
							highlighted=new_entry_highlighted,
							close_handler=new_entry_close_handler,
							editing_new=true
						)
					}
				}
			} else {
				view! { ctx, }
			})
		}
		datalist(id="event_entry_types") {
			Keyed(
				iterable=read_entry_types_signal,
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
		datalist(id="event_tags") {
			Keyed(
				iterable=read_tags_signal,
				key=|tag| tag.id.clone(),
				view=|ctx, tag| {
					let tag_name = tag.name;
					view! {
						ctx,
						option(value=tag_name)
					}
				}
			)
		}
		datalist(id="editor_names") {
			Keyed(
				iterable=read_available_editors,
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
