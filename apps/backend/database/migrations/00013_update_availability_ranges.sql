-- Add blocked_ranges support to existing availability_ranges table
-- (table already exists from initial schema, this migration adds blocking semantics)

-- Add reason field for block type (maintenance, personal_use, etc.)
ALTER TABLE availability_ranges
  ADD COLUMN IF NOT EXISTS reason VARCHAR(100),
  ADD COLUMN IF NOT EXISTS blocked_by UUID REFERENCES users(id);

-- Index for fast date-range lookups
CREATE INDEX IF NOT EXISTS idx_availability_ranges_dates
  ON availability_ranges(property_id, start_date, end_date);

-- RLS: owners manage their ranges; anyone can read
ALTER TABLE availability_ranges ENABLE ROW LEVEL SECURITY;

CREATE POLICY IF NOT EXISTS "Owners can manage availability ranges" ON availability_ranges
  USING (
    EXISTS (SELECT 1 FROM properties WHERE properties.id = property_id AND properties.owner_id = auth.uid())
  );

CREATE POLICY IF NOT EXISTS "Anyone can read availability ranges" ON availability_ranges
  FOR SELECT USING (true);
