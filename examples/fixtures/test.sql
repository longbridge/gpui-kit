SELECT *
FROM users
WHERE email ilike '%test%'
  AND deleted_at IS NOT NULL
LIMIT 1;
