-- Extension required for gen_random_uuid() used in sessions.id generation.
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- User role model.
CREATE TYPE user_role AS ENUM ('admin', 'regular');
-- Chat kind model.
CREATE TYPE chat_kind AS ENUM ('with_self', 'private', 'group', 'channel');
-- Member role inside chat.
CREATE TYPE chat_role AS ENUM ('owner', 'moderator', 'member');

-- Registered application users.
CREATE TABLE users (
    id               int PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    alias            VARCHAR(30) NOT NULL UNIQUE,
    display_name     VARCHAR(30) NOT NULL,
    password_hash    TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL,
    role             user_role NOT NULL,
    bio              VARCHAR(255),
    invited_by       int REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL
);

-- Singleton row with global system pointers (currently origin user id).
CREATE TABLE system_state (
    singleton       BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    origin_user_id  int NOT NULL UNIQUE REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT
);

-- Logical chat entities for all chat kinds.
CREATE TABLE chats (
    id              bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    display_name    VARCHAR(50),
    description     VARCHAR(255),
    kind            chat_kind NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL
);

-- Canonical participant pair for private chats, used for uniqueness and integrity.
CREATE TABLE private_chats (
    chat_id        bigint PRIMARY KEY REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
    user_id_low    int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    user_id_high   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE RESTRICT,
    CONSTRAINT private_chat_pair_order CHECK (user_id_low < user_id_high),
    CONSTRAINT private_chat_pair_unique UNIQUE (user_id_low, user_id_high)
);

-- Active login sessions and token material.
CREATE TABLE sessions (
    id                        uuid PRIMARY KEY,
    user_id                   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
    ip                        inet NOT NULL,
    first_seen_at             TIMESTAMPTZ NOT NULL,
    last_seen_at              TIMESTAMPTZ NOT NULL,
    device_name               VARCHAR(100),
    os_version                VARCHAR(100),
    app_version               VARCHAR(100),
    refresh_token_hash        BYTEA NOT NULL,
    refresh_token_expires_at  TIMESTAMPTZ NOT NULL,
    access_token_hash         BYTEA NOT NULL,
    access_token_expires_at   TIMESTAMPTZ NOT NULL,
    refresh_counter           int NOT NULL
);

-- User membership in chats with per-chat role.
CREATE TABLE chats_members (
    chat_id   bigint NOT NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
    user_id   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
    role      chat_role NOT NULL,
    CONSTRAINT chat_user_pkey PRIMARY KEY (user_id, chat_id)
);

-- Uploaded resources metadata.
CREATE TABLE resources (
    id                      bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    uploaded_by_user_id     INTEGER REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
    url                     VARCHAR(255) NOT NULL
);

-- Chat messages (text and optional attachments/replies).
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

-- Supports chat membership checks by chat id.
CREATE INDEX idx_chats_members_chat_id_user_id ON chats_members(chat_id, user_id);
-- Supports message pagination/polling by chat and message id.
CREATE INDEX idx_messages_chat_id_message_id ON messages(chat_id, id);
-- Supports session trimming by user and newest access expiration.
CREATE INDEX idx_sessions_user_id_access_token_expires_at_desc ON sessions(user_id, access_token_expires_at DESC);

-- Ensures private_chats row always points to a private chat entity.
CREATE OR REPLACE FUNCTION check_private_chat_kind() RETURNS trigger
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM chats
        WHERE id = NEW.chat_id AND kind = 'private'
    ) THEN
        RAISE EXCEPTION 'private_chats.chat_id % must reference chats.kind=private', NEW.chat_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Ensures only canonical pair members can be inserted into private chats.
CREATE OR REPLACE FUNCTION check_private_chat_member() RETURNS trigger
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM chats
        WHERE id = NEW.chat_id AND kind = 'private'
    ) THEN
        -- Not a private chat, this check is irrelevant.
        RETURN NEW;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM private_chats
        WHERE chat_id = NEW.chat_id
          AND NEW.user_id IN (user_id_low, user_id_high)
    ) THEN
        RAISE EXCEPTION 'user % is not allowed in private chat %', NEW.user_id, NEW.chat_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Validates private chat metadata updates/inserts.
CREATE TRIGGER private_chats_kind_guard
BEFORE INSERT OR UPDATE ON private_chats
FOR EACH ROW EXECUTE FUNCTION check_private_chat_kind();

-- Validates membership writes that can change chat/user relation.
CREATE TRIGGER private_chat_member_guard
BEFORE INSERT OR UPDATE OF chat_id, user_id ON chats_members
FOR EACH ROW EXECUTE FUNCTION check_private_chat_member();
