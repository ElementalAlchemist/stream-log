// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::SubscriptionManager;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use tide::Server;

mod v1;
use v1::add_routes as add_v1_routes;

pub fn add_routes(
	app: &mut Server<()>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> miette::Result<()> {
	add_v1_routes(app, db_connection_pool, subscription_manager)
}
