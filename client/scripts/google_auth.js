function launch_google_auth_flow(client_id) {
	const google_script = document.createElement("script");
	google_script.src = "https://accounts.google.com/gsi/client";
	google_script.addEventListener("load", (_event) => {
		google.accounts.id.initialize({
			client_id: client_id,
			ux_mode: "redirect"
		});
		google.accounts.id.renderButton(document.body, {});
		google.accounts.id.prompt();
	});
	document.head.appendChild(google_script);
}