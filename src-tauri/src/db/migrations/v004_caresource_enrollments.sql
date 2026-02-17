-- V004: Create CareSource enrollments for all existing clients
-- All current clients are CareSource book of business members.

INSERT INTO enrollments (id, client_id, carrier_id, status_code)
SELECT
    lower(hex(randomblob(4))) || '-' ||
    lower(hex(randomblob(2))) || '-4' ||
    substr(lower(hex(randomblob(2))),2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) || substr(lower(hex(randomblob(2))),2) || '-' ||
    lower(hex(randomblob(6))),
    c.id,
    'carrier-caresource',
    'ACTIVE'
FROM clients c
WHERE c.is_active = 1
  AND NOT EXISTS (
    SELECT 1 FROM enrollments e
     WHERE e.client_id = c.id
       AND e.is_active = 1
  );
