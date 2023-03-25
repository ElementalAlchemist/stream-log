CREATE TABLE event_editors (
	event TEXT REFERENCES events,
	editor TEXT REFERENCES users,
	PRIMARY KEY (event, editor)
);