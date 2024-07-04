ALTER TABLE  guild ADD goodbye_channel BIGINT;

ALTER TABLE  guild ADD goodbye_messages TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[];