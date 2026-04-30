-- Convert monetary columns from REAL/DOUBLE PRECISION to BIGINT (integer subunits)
-- 1 USD = 100,000,000 subunits (8 decimal places)
-- markup_ratio: 1.0 = 10,000 basis points

ALTER TABLE api_keys ALTER COLUMN budget_monthly TYPE BIGINT USING ROUND(COALESCE(budget_monthly, 0) * 100000000);
ALTER TABLE usage_records ALTER COLUMN cost TYPE BIGINT USING ROUND(cost * 100000000);
ALTER TABLE channels ALTER COLUMN markup_ratio TYPE BIGINT USING ROUND(markup_ratio * 10000);
ALTER TABLE channels ALTER COLUMN balance TYPE BIGINT USING ROUND(COALESCE(balance, 0) * 100000000);
ALTER TABLE channel_models ALTER COLUMN markup_ratio TYPE BIGINT USING ROUND(markup_ratio * 10000);
ALTER TABLE accounts ALTER COLUMN balance TYPE BIGINT USING ROUND(balance * 100000000);
ALTER TABLE accounts ALTER COLUMN threshold TYPE BIGINT USING ROUND(threshold * 100000000);
ALTER TABLE transactions ALTER COLUMN amount TYPE BIGINT USING ROUND(amount * 100000000);
ALTER TABLE transactions ALTER COLUMN balance_after TYPE BIGINT USING ROUND(balance_after * 100000000);
