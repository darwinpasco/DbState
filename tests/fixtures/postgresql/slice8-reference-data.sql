-- DbState Slice 8 reference-data fixture.
-- Test-only. Apply only to a disposable local PostgreSQL database.

DROP SCHEMA IF EXISTS dbstate_ref CASCADE;
CREATE SCHEMA dbstate_ref;

CREATE TABLE dbstate_ref.payment_methods (
    code text PRIMARY KEY,
    name text NOT NULL,
    is_active boolean NOT NULL,
    sort_order integer NOT NULL,
    updated_at timestamp without time zone,
    secret_note text
);

INSERT INTO dbstate_ref.payment_methods (
    code,
    name,
    is_active,
    sort_order,
    updated_at,
    secret_note
) VALUES
    ('CASH', 'Cash', true, 10, '2026-01-01 00:00:00', 'slice8-db-secret-cash'),
    ('QRPH', 'QRPh Live', true, 20, '2026-01-02 00:00:00', 'slice8-db-secret-qrph'),
    ('CARD', 'Card', true, 30, '2026-01-03 00:00:00', 'slice8-db-secret-card');
