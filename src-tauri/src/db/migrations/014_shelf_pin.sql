-- Files on the shelf can be pinned so they stay past the clean-up.
ALTER TABLE shelf_items ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
