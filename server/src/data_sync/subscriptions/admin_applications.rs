use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::Application as ApplicationDb;
use crate::schema::applications;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use base64::engine::general_purpose::STANDARD_NO_PAD as base64_engine;
use base64::Engine;
use diesel::prelude::*;
use rand::random;
use stream_log_shared::messages::admin::{AdminApplicationData, AdminApplicationUpdate, Application};
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_applications(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminApplications,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let applications: QueryResult<Vec<ApplicationDb>> = applications::table
		.filter(applications::auth_key.is_not_null())
		.load(&mut *db_connection);
	let applications: Vec<Application> = match applications {
		Ok(mut apps) => apps.drain(..).map(|app| app.into()).collect(),
		Err(error) => {
			tide::log::error!(
				"A database error occurred loading applications for admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminApplications,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_applications_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminApplications(
		applications,
	)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_applications_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminApplicationUpdate,
	conn_update_tx: Sender<ConnectionUpdate>,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_applications(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminApplicationUpdate::UpdateApplication(mut application) => {
			if application.id.is_empty() {
				application.id = cuid2::create_id();
				let auth_key = generate_application_auth_key();
				let db_application = ApplicationDb {
					id: application.id.clone(),
					name: application.name.clone(),
					auth_key: Some(auth_key.clone()),
					read_log: application.read_log,
					write_links: application.write_links,
					creation_user: user.id.clone(),
				};

				let insert_result: QueryResult<_> = {
					let mut db_connection = db_connection.lock().await;
					diesel::insert_into(applications::table)
						.values(db_application)
						.execute(&mut *db_connection)
				};
				if let Err(error) = insert_result {
					tide::log::error!("A database error occurred adding a new application: {}", error);
					return;
				}

				let subscription_manager = subscription_manager.lock().await;
				let message = SubscriptionData::AdminApplicationsUpdate(AdminApplicationData::UpdateApplication(
					application.clone(),
				));
				let send_result = subscription_manager.broadcast_admin_applications_message(message).await;
				if let Err(error) = send_result {
					tide::log::error!("Failed to send new application to admin subscription: {}", error);
				}

				let message =
					FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::AdminApplicationsUpdate(
						AdminApplicationData::ShowApplicationAuthKey(application, auth_key),
					)));
				let send_result = conn_update_tx.send(ConnectionUpdate::SendData(Box::new(message))).await;
				if let Err(error) = send_result {
					tide::log::error!("Failed to send application auth key message: {}", error);
				}
			} else {
				let update_result: QueryResult<_> = {
					let mut db_connection = db_connection.lock().await;
					diesel::update(applications::table)
						.filter(applications::id.eq(&application.id))
						.set((
							applications::name.eq(&application.name),
							applications::read_log.eq(application.read_log),
							applications::write_links.eq(application.write_links),
						))
						.execute(&mut *db_connection)
				};
				if let Err(error) = update_result {
					tide::log::error!("A database error occurred updating an application: {}", error);
					return;
				}

				let subscription_manager = subscription_manager.lock().await;
				let message =
					SubscriptionData::AdminApplicationsUpdate(AdminApplicationData::UpdateApplication(application));
				let send_result = subscription_manager.broadcast_admin_applications_message(message).await;
				if let Err(error) = send_result {
					tide::log::error!("Failed to send application update to admin subscription: {}", error);
				}
			}
		}
		AdminApplicationUpdate::ResetAuthToken(application) => {
			let new_auth_key = generate_application_auth_key();
			let update_result = {
				let mut db_connection = db_connection.lock().await;
				diesel::update(applications::table)
					.filter(applications::id.eq(&application.id))
					.set(applications::auth_key.eq(&new_auth_key))
					.execute(&mut *db_connection)
			};
			if let Err(error) = update_result {
				tide::log::error!("A database error occurred resetting an application auth key: {}", error);
				return;
			}

			let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::AdminApplicationsUpdate(
				AdminApplicationData::ShowApplicationAuthKey(application, new_auth_key),
			)));
			let send_result = conn_update_tx.send(ConnectionUpdate::SendData(Box::new(message))).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send application auth key message: {}", error);
			}
		}
		AdminApplicationUpdate::RevokeApplication(application) => {
			let update_result = {
				let mut db_connection = db_connection.lock().await;
				let null_auth_key: Option<String> = None;
				diesel::update(applications::table)
					.filter(applications::id.eq(&application.id))
					.set(applications::auth_key.eq(null_auth_key))
					.execute(&mut *db_connection)
			};
			if let Err(error) = update_result {
				tide::log::error!("A database error occurred revoking an application: {}", error);
				return;
			}

			let subscription_manager = subscription_manager.lock().await;
			let message =
				SubscriptionData::AdminApplicationsUpdate(AdminApplicationData::RevokeApplication(application));
			let send_result = subscription_manager.broadcast_admin_applications_message(message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send application revokation to admin subscription: {}", error);
			}
		}
	}
}

/// Generates a new authorization key for an application.
fn generate_application_auth_key() -> String {
	// We want to generate a reasonable but still pretty secure (unlikely to be guessed) key with collisions as unlikely
	// as we can make them. To that end, we start with a new CUID2 ID, as it is fairly random on its own and provides
	// the collision resistance we want, and we append a decent amount of random data to it.

	let id = cuid2::create_id();
	let random_number: u128 = random();
	let random_data = base64_engine.encode(random_number.to_ne_bytes());
	format!("{}.{}", id, random_data)
}
