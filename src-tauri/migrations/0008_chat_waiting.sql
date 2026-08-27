-- Chats that stopped to ask something.
--
-- A background task ends the same way whether it finished the job or stopped
-- to ask: the process exits, the answer lands in a chat nobody is looking at.
-- When that answer was a question, the exchange is open and the work it was
-- about is waiting with it — silently, which is the whole problem.
--
-- This is on the chat rather than on the run on purpose. The run succeeded:
-- it did what it was asked and reported back. What is unfinished is the
-- conversation, and a conversation is a chat. A sixth run state would also
-- have to be swept, replayed and reasoned about, all to describe something the
-- process never knew.
--
-- `waiting_since` doubles as the flag: null means nothing is pending. Storing
-- the moment rather than a boolean costs nothing and answers the question a
-- banner actually raises — how long has this been sitting here.

ALTER TABLE chat ADD COLUMN waiting_since TEXT;

-- The banner asks "is anything waiting?" on every screen, so the lookup is by
-- profile and by presence, not by chat.
CREATE INDEX chat_waiting ON chat (profile_id) WHERE waiting_since IS NOT NULL;
