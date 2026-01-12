# Troubleshooting Database Connection Issues

## Common Connection Errors

### Error: "Connection Refused"

**Symptoms:**
- Application cannot connect to database
- Error message: `ECONNREFUSED` or `Connection refused`

**Possible Causes:**
1. Database server not running
2. Wrong port number
3. Firewall blocking connection
4. Database not listening on correct interface

**Solutions:**

#### Check if database is running
```bash
# PostgreSQL
sudo systemctl status postgresql

# MySQL
sudo systemctl status mysql

# MongoDB
sudo systemctl status mongod
```

#### Verify connection parameters
```bash
# Test PostgreSQL connection
psql -h localhost -p 5432 -U postgres

# Test MySQL connection
mysql -h localhost -P 3306 -u root -p
```

#### Check firewall rules
```bash
# Ubuntu/Debian
sudo ufw status
sudo ufw allow 5432/tcp

# CentOS/RHEL
sudo firewall-cmd --list-all
sudo firewall-cmd --add-port=5432/tcp --permanent
```

### Error: "Too Many Connections"

**Symptoms:**
- New connections fail
- Error: `FATAL: too many connections for role`

**Solutions:**

1. **Increase max_connections:**
```sql
-- PostgreSQL
ALTER SYSTEM SET max_connections = 200;
-- Restart required
```

2. **Implement connection pooling:**
```javascript
// Node.js with pg-pool
const { Pool } = require('pg');
const pool = new Pool({
  max: 20,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
});
```

3. **Kill idle connections:**
```sql
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'idle'
AND state_change < current_timestamp - INTERVAL '5 minutes';
```

### Error: "Authentication Failed"

**Quick fixes:**
1. Verify username and password
2. Check `pg_hba.conf` authentication method
3. Ensure user has proper permissions
4. Reset password if necessary

```sql
-- PostgreSQL password reset
ALTER USER myuser WITH PASSWORD 'newpassword';

-- MySQL password reset
ALTER USER 'myuser'@'localhost' IDENTIFIED BY 'newpassword';
```

## Performance Optimization

### Slow Queries

Use query analysis tools:
```sql
-- PostgreSQL
EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'user@example.com';

-- MySQL
EXPLAIN SELECT * FROM users WHERE email = 'user@example.com';
```

### Connection Pool Sizing

Rule of thumb: `connections = ((core_count * 2) + effective_spindle_count)`

For a 4-core machine with SSD: `(4 * 2) + 1 = 9 connections`
