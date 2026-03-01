CREATE TYPE user_role AS ENUM ('admin', 'regular');
CREATE TYPE chat_kind AS ENUM ('with_self', 'private', 'group', 'channel');
CREATE TYPE chat_role AS ENUM ('owner', 'moderator', 'member');

CREATE TABLE users (
    id               int PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    alias            VARCHAR(30) NOT NULL UNIQUE,
    display_name     VARCHAR(30) NOT NULL,
    password_salt    BYTEA NOT NULL,
    password_hash    BYTEA NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    role             user_role NOT NULL,
    bio              VARCHAR(255),
    invited_by       int REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL
);

CREATE TABLE system_state (
    singleton       BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    origin_user_id  int NOT NULL UNIQUE REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT
);

CREATE TABLE chats (
    id              bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    display_name    VARCHAR(50),
    description     VARCHAR(255),
    kind            chat_kind NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL
);

CREATE TABLE sessions (
    id                        uuid PRIMARY KEY,
    user_id                   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
    ip                        inet NOT NULL,
    first_seen_at             TIMESTAMPTZ NOT NULL,
    last_seen_at              TIMESTAMPTZ NOT NULL,
    device_name               VARCHAR(100),
    os_version                VARCHAR(100),
    app_version               VARCHAR(100),
    refresh_token             BYTEA NOT NULL,
    refresh_token_expires_at  TIMESTAMPTZ NOT NULL,
    access_token              BYTEA NOT NULL,
    access_token_expires_at   TIMESTAMPTZ NOT NULL,
    refresh_counter           int NOT NULL
);

CREATE TABLE chats_members (
    chat_id   bigint NOT NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
    user_id   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
    role      chat_role NOT NULL,
    CONSTRAINT chat_user_pkey PRIMARY KEY (user_id, chat_id)
);

CREATE TABLE resources (
    id                      bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uploaded_by_user_id     INTEGER NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
    url                     VARCHAR(255) NOT NULL
);

CREATE TABLE messages (
    id           bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    chat_id      bigint NOT NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
    user_id      int REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
    text         VARCHAR(4096),
    reply_to     bigint REFERENCES messages(id) ON UPDATE CASCADE ON DELETE SET NULL,
    resource_id  bigint REFERENCES resources(id) ON UPDATE CASCADE ON DELETE NO ACTION,
    created_at   TIMESTAMPTZ NOT NULL,
    edited_at    TIMESTAMPTZ
);
