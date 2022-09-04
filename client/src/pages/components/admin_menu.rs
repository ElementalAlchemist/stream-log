use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub enum AdminMenuClicked {
	ManageEvents,
	ManageUsers,
	ManagePermissionGroups,
	AssignUsersToPermissionGroups,
	Exit,
}

#[derive(Prop)]
pub struct AdminMenuProps {
	click_signal: RcSignal<Option<AdminMenuClicked>>,
}

#[component]
pub fn AdminMenu<G: Html>(ctx: Scope, props: AdminMenuProps) -> View<G> {
	let click_signal = props.click_signal;
	view! {
		ctx,
		ul(id="admin_menu") {
			li {
				a(
					class="click",
					on:click={
						let click_signal = click_signal.clone();
						move |_| click_signal.set(Some(AdminMenuClicked::ManageEvents))
					}
				) {
					"Manage Events"
				}
			}
			li {
				a(
					class="click",
					on:click={
						let click_signal = click_signal.clone();
						move |_| click_signal.set(Some(AdminMenuClicked::ManageUsers))
					}
				) {
					"Manage Users"
				}
			}
			li {
				a(
					class="click",
					on:click={
						let click_signal = click_signal.clone();
						move |_| click_signal.set(Some(AdminMenuClicked::ManagePermissionGroups))
					}
				) {
					"Manage Permission Groups"
				}
			}
			li {
				a(
					class="click",
					on:click={
						let click_signal = click_signal.clone();
						move |_| click_signal.set(Some(AdminMenuClicked::AssignUsersToPermissionGroups))
					}
				) {
					"Assign Users to Permission Groups"
				}
			}
			li {
				a(
					class="click",
					on:click=move |_| click_signal.set(Some(AdminMenuClicked::Exit))
				) {
					"Exit"
				}
			}
		}
	}
}
