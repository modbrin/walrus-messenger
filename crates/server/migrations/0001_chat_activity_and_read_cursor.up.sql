ALTER TABLE chats
    ADD COLUMN last_message_id bigint REFERENCES messages(id) ON UPDATE CASCADE ON DELETE SET NULL,
    ADD COLUMN last_message_at TIMESTAMPTZ;

ALTER TABLE chats_members
    ADD COLUMN last_read_message_id bigint REFERENCES messages(id) ON UPDATE CASCADE ON DELETE SET NULL;

WITH latest_messages AS (
    SELECT DISTINCT ON (chat_id)
        chat_id,
        id AS message_id,
        created_at
    FROM messages
    ORDER BY chat_id, id DESC
)
UPDATE chats
SET
    last_message_id = latest_messages.message_id,
    last_message_at = latest_messages.created_at
FROM latest_messages
WHERE chats.id = latest_messages.chat_id;

CREATE INDEX idx_chats_last_message_at_id_desc
    ON chats(last_message_at DESC, id DESC);
