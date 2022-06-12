CREATE TABLE user_permissions (
	user_id TEXT REFERENCES users,
	permission_group TEXT REFERENCES permission_groups,
	PRIMARY KEY (user_id, permission_group)
);