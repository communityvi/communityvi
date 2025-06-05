CREATE TABLE medium
(
	uuid                          text    not null,
	name                          text    not null,
	version                       integer not null,
	length_ms                     integer not null,
	playback_state                text    not null,
	playback_state_start_time_ms  integer,
	playback_state_at_position_ms integer,
	constraint check_non_negative_length
		check (length_ms >= 0),
	constraint check_valid_playback_state
		check (playback_state = 'playing'
				   and playback_state_start_time_ms is not null
				   and playback_state_at_position_ms is null
			or playback_state = 'paused'
				   and playback_state_start_time_ms is null
				   and playback_state_at_position_ms >= 0)
);

CREATE TABLE room
(
	uuid        text not null
		constraint room_pk
			primary key,
	name        text not null
		constraint room_name_uq
			unique,
	medium_uuid text
		constraint room_medium__fk
			references medium (uuid)
			on delete set null
);

CREATE TABLE user
(
	uuid text not null
		constraint user_pk
			primary key,
	name text
		constraint user_name_uq
			unique
);

CREATE TABLE room_user
(
	room_uuid integer not null
		constraint room_user_room__fk
			references room
			on delete cascade,
	user_uuid integer not null
		constraint room_user_user__fk
			references user
			on delete cascade,
	constraint room_user_pk
		primary key (room_uuid, user_uuid)
);

CREATE TABLE chat_message
(
	uuid       text                               not null
		constraint chat_message_pk
			primary key,
	room_uuid  text                               not null
		constraint chat_message_room__fk
			references room (uuid)
			on delete cascade,
	user_uuid  text                               not null
		constraint chat_message_user__fk
			references user (uuid),
	user_name  text                               not null,
	message    text                               not null,
	created_at datetime default current_timestamp not null,
	constraint check_non_empty_message
		check (length(message) > 0)
);
