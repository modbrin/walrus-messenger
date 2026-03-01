DROP TRIGGER IF EXISTS private_chat_member_guard ON chats_members;
DROP TRIGGER IF EXISTS private_chats_kind_guard ON private_chats;
DROP FUNCTION IF EXISTS check_private_chat_member;
DROP FUNCTION IF EXISTS check_private_chat_kind;

DROP INDEX IF EXISTS idx_sessions_user_id_access_token_expires_at_desc;
DROP INDEX IF EXISTS idx_messages_chat_id_message_id;
DROP INDEX IF EXISTS idx_chats_members_chat_id_user_id;

DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS resources;
DROP TABLE IF EXISTS chats_members;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS private_chats;
DROP TABLE IF EXISTS chats;
DROP TABLE IF EXISTS system_state;
DROP TABLE IF EXISTS users;

DROP TYPE IF EXISTS chat_role;
DROP TYPE IF EXISTS chat_kind;
DROP TYPE IF EXISTS user_role;
