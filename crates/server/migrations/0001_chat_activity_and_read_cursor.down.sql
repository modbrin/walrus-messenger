DROP INDEX IF EXISTS idx_chats_last_message_at_id_desc;

ALTER TABLE chats_members
    DROP COLUMN IF EXISTS last_read_message_id;

ALTER TABLE chats
    DROP COLUMN IF EXISTS last_message_at,
    DROP COLUMN IF EXISTS last_message_id;
