DROP VIEW IF EXISTS dbstate_slice16.active_accounts;
DROP INDEX IF EXISTS dbstate_slice16.sample_accounts_code_idx;
DROP TABLE IF EXISTS dbstate_slice16.sample_accounts;
DROP SEQUENCE IF EXISTS dbstate_slice16.account_number_seq;
DROP TYPE IF EXISTS dbstate_slice16.account_status;
DROP SCHEMA IF EXISTS dbstate_slice16;

CREATE SCHEMA dbstate_slice16;

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE dbstate_slice16.account_status AS ENUM (
    'active',
    'closed'
);

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
    status dbstate_slice16.account_status NOT NULL DEFAULT 'active'
);

CREATE UNIQUE INDEX sample_accounts_code_idx
    ON dbstate_slice16.sample_accounts USING btree (account_code);

CREATE VIEW dbstate_slice16.active_accounts AS
 SELECT sample_accounts.account_id,
    sample_accounts.account_code
   FROM dbstate_slice16.sample_accounts
  WHERE sample_accounts.status = 'active';
