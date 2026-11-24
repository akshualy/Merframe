ALTER TABLE snapshots ADD COLUMN account TEXT;
ALTER TABLE trades ADD COLUMN account TEXT;
ALTER TABLE relic_openings ADD COLUMN account TEXT;

CREATE INDEX snapshots_account ON snapshots (account, taken_at);
CREATE INDEX trades_account ON trades (account, at);
CREATE INDEX relic_openings_account ON relic_openings (account, at);

ALTER TABLE snapshots DROP COLUMN ducats;
ALTER TABLE snapshots DROP COLUMN aya;

CREATE INDEX deltas_item_type ON deltas (item_type);
