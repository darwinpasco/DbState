DROP VIEW IF EXISTS dbstate_slice16.active_accounts;
DROP INDEX IF EXISTS dbstate_slice16.sample_accounts_code_idx;
DROP AGGREGATE IF EXISTS dbstate_slice16.account_codes(text);
DROP FUNCTION IF EXISTS dbstate_slice16._account_codes(text, text);
DROP TABLE IF EXISTS dbstate_slice16.payment CASCADE;
DROP TABLE IF EXISTS dbstate_slice16.sample_accounts;
DROP SEQUENCE IF EXISTS dbstate_slice16.account_number_seq;
DROP DOMAIN IF EXISTS dbstate_slice16.account_year;
DROP TYPE IF EXISTS dbstate_slice16.account_status;
DROP SCHEMA IF EXISTS dbstate_slice16;

CREATE SCHEMA dbstate_slice16;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE dbstate_slice16.account_status AS ENUM (
    'active',
    'closed'
);

CREATE DOMAIN dbstate_slice16.account_year AS integer
    CHECK (VALUE >= 1901 AND VALUE <= 2155);

CREATE SEQUENCE dbstate_slice16.account_number_seq
    AS bigint
    START WITH 100
    INCREMENT BY 5
    MINVALUE 100
    MAXVALUE 9223372036854775807
    CACHE 1
    NO CYCLE;

CREATE TABLE dbstate_slice16.sample_accounts (
    account_id integer NOT NULL,
    account_code text NOT NULL,
    opened_year dbstate_slice16.account_year,
    status dbstate_slice16.account_status NOT NULL DEFAULT 'active'
);

CREATE TABLE dbstate_slice16.payment (
    payment_id integer NOT NULL,
    payment_date timestamp without time zone NOT NULL
) PARTITION BY RANGE (payment_date);

CREATE UNIQUE INDEX sample_accounts_code_idx
    ON dbstate_slice16.sample_accounts USING btree (account_code);

CREATE FUNCTION dbstate_slice16._account_codes(state text, value text)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE WHEN state IS NULL THEN value ELSE state || ',' || value END;
$$;

CREATE AGGREGATE dbstate_slice16.account_codes(text) (
    SFUNC = dbstate_slice16._account_codes,
    STYPE = text
);

CREATE VIEW dbstate_slice16.active_accounts AS
 SELECT sample_accounts.account_id,
    sample_accounts.account_code
   FROM dbstate_slice16.sample_accounts
  WHERE sample_accounts.status = 'active';
