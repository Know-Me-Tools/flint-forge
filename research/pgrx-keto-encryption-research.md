# Research Report: pgrx, Ory Keto PostgreSQL Storage, and PostgreSQL Encryption Patterns

> **Research Date:** 2026-06-30  
> **Scope:** Implementation details, SQL code examples, and architectural patterns for three advanced PostgreSQL topics.

---

## Table of Contents

1. [pgrx Framework for PostgreSQL Extensions](#1-pgrx-framework-for-postgresql-extensions)
2. [Ory Keto with PostgreSQL Storage](#2-ory-keto-with-postgresql-storage)
3. [Row-Level Security and Column-Level Encryption in PostgreSQL](#3-row-level-security-and-column-level-encryption-in-postgresql)

---

## 1. pgrx Framework for PostgreSQL Extensions

### 1.1 Overview

`pgrx` is a Rust framework for developing PostgreSQL extensions. It provides memory safety, automatic type conversion, automatic SQL schema generation, and an integrated test workflow. It supports PostgreSQL 13 through 18 (and 19 beta), allowing the same Rust codebase to target multiple versions via feature gating.

**Key capabilities:**
- SQL-callable functions via `#[pg_extern]`
- Custom types (`#[derive(PostgresType)]`), enums (`#[derive(PostgresEnum)]`), and composite types
- Triggers (`#[pg_trigger]`)
- Set-returning functions (`SetOfIterator`, `TableIterator`)
- Server Programming Interface (SPI) for executing SQL from within the extension
- Background workers and hooks (executor, planner, transaction, subtransaction)
- Direct access to PostgreSQL internals via `pgrx::pg_sys`

### 1.2 Development Environment Setup

```bash
# Install the pgrx toolchain
cargo install --locked cargo-pgrx

# Download and compile supported PostgreSQL versions
cargo pgrx init

# Create a new extension project
cargo pgrx new my_extension
cd my_extension

# Run extension interactively in psql
cargo pgrx run pg18

# Run tests across PostgreSQL versions
cargo pgrx test

# Generate installation package
cargo pgrx package
```

### 1.3 Creating SQL-Callable Functions in Rust

The `#[pg_extern]` macro exposes Rust functions to PostgreSQL. pgrx automatically generates SQL wrappers.

```rust
use pgrx::prelude::*;

pgrx::pg_module_magic!();

#[pg_extern]
fn hello_my_extension() -> &'static str {
    "Hello, my_extension"
}

#[pg_extern]
fn to_title(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[pg_extern]
fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}
```

**Auto-generated SQL:**

```sql
CREATE OR REPLACE FUNCTION to_title(input text)
RETURNS text
STRICT
LANGUAGE c AS 'MODULE_PATHNAME', 'to_title_wrapper';

CREATE OR REPLACE FUNCTION add_numbers(a integer, b integer)
RETURNS integer
STRICT
LANGUAGE c AS 'MODULE_PATHNAME', 'add_numbers_wrapper';
```

**Type Mapping (Rust ↔ PostgreSQL):**

| Rust Type | PostgreSQL Type | Notes |
|-----------|-----------------|-------|
| `i32` | `integer` | Scalar |
| `i64` | `bigint` | Scalar |
| `String` | `text` | Allocated |
| `&str` | `text` | Zero-copy borrow |
| `Vec<u8>` | `bytea` | Binary data |
| `Option<T>` | `T` or `NULL` | Nullable |
| `Vec<T>` | `T[]` | Arrays |
| `pgrx::Json(serde_json::Value)` | `json` | JSON |
| `pgrx::JsonB(serde_json::Value)` | `jsonb` | JSONB |

### 1.4 Custom Types

```rust
use pgrx::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(PostgresType, Serialize, Deserialize, Debug)]
pub struct UserProfile {
    username: String,
    age: i32,
    preferences: std::collections::HashMap<String, String>,
}

#[pg_extern]
fn create_user(username: String, age: i32) -> UserProfile {
    UserProfile {
        username,
        age,
        preferences: std::collections::HashMap::new(),
    }
}
```

By default, `PostgresType` uses **CBOR** encoding for on-disk storage and **JSON** for the human-readable text representation. Binary protocol support is available via `#[pg_binary_protocol]`.

### 1.5 Set-Returning Functions

```rust
use pgrx::prelude::*;
use pgrx::iter::SetOfIterator;

#[pg_extern]
fn generate_series(start: i32, end: i32) -> SetOfIterator<'static, i32> {
    SetOfIterator::new((start..=end).collect::<Vec<i32>>().into_iter())
}

// Returns TABLE(...)
use pgrx::iter::TableIterator;

#[pg_extern]
fn get_users() -> TableIterator<'static, (name!(id, i32), name!(name, String))> {
    let users = vec![
        (1, "Alice".to_string()),
        (2, "Bob".to_string()),
    ];
    TableIterator::new(users.into_iter())
}
```

### 1.6 Interacting with PostgreSQL System Catalogs from an Extension

pgrx provides direct (unsafe) access to PostgreSQL internals through the `pgrx::pg_sys` module, which is a bindgen-generated interface to PostgreSQL's C headers.

**Key patterns:**

```rust
use pgrx::prelude::*;
use pgrx::pg_sys;

#[pg_extern]
fn get_relation_name(oid: pg_sys::Oid) -> Option<String> {
    unsafe {
        // Get relation name from OID using pg_sys function
        let name_ptr = pg_sys::get_rel_name(oid);
        if name_ptr.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(name_ptr).to_string_lossy().into_owned())
        }
    }
}

#[pg_extern]
fn get_current_database_name() -> String {
    unsafe {
        let db_name = pg_sys::get_database_name(pg_sys::MyDatabaseId);
        std::ffi::CStr::from_ptr(db_name)
            .to_string_lossy()
            .into_owned()
    }
}
```

**System catalog structs available in `pg_sys`:**
- `FormData_pg_class` — table metadata
- `FormData_pg_attribute` — column metadata
- `FormData_pg_type` — type definitions
- `FormData_pg_namespace` — schema information
- `FormData_pg_trigger` — trigger definitions
- `FormData_pg_proc` — function/procedure metadata

**Accessing catalog tables via SPI:**

```rust
use pgrx::spi::Spi;

#[pg_extern]
fn list_user_tables() -> Vec<String> {
    Spi::connect(|client| {
        let mut result = Vec::new();
        let query = "SELECT relname FROM pg_class WHERE relkind = 'r' AND relnamespace = 'public'::regnamespace";
        client.select(query, None, None).unwrap().for_each(|row| {
            let name: Option<String> = row.get("relname");
            if let Some(n) = name {
                result.push(n);
            }
        });
        result
    })
}
```

### 1.7 Creating Event Triggers Programmatically

Event triggers fire on DDL events (`CREATE TABLE`, `ALTER`, `DROP`, etc.). In pgrx, you can define trigger functions with `#[pg_trigger]` and use SPI to inspect the command being executed.

```rust
use pgrx::prelude::*;

#[pg_trigger]
fn ddl_event_trigger(trigger: &PgTrigger) -> PgTriggerResult {
    // Access trigger data
    let event = trigger.event(); // e.g., "ddl_command_start"
    let tag = trigger.tag();     // e.g., "CREATE TABLE"
    
    // Log the DDL operation
    ereport!(INFO,
        PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
        format!("DDL event: {}, tag: {}", event, tag)
    );
    
    trigger.new()
}
```

**Registering the event trigger (SQL):**

```sql
-- Event trigger function (generated by pgrx)
CREATE OR REPLACE FUNCTION ddl_event_trigger()
RETURNS event_trigger
LANGUAGE c AS 'MODULE_PATHNAME', 'ddl_event_trigger_wrapper';

-- Register on all DDL command ends
CREATE EVENT TRIGGER log_ddl_changes
ON ddl_command_end
EXECUTE FUNCTION ddl_event_trigger();
```

**Inspecting DDL details via `pg_event_trigger_ddl_commands()`:**

```rust
#[pg_trigger]
fn ddl_command_end_trigger(trigger: &PgTrigger) -> PgTriggerResult {
    Spi::connect(|client| {
        let query = "SELECT objid, object_type, schema_name, object_identity 
                     FROM pg_event_trigger_ddl_commands()";
        client.select(query, None, None).unwrap().for_each(|row| {
            let obj_type: Option<String> = row.get("object_type");
            let identity: Option<String> = row.get("object_identity");
            ereport!(INFO,
                PgSqlErrorCode::ERRCODE_SUCCESSFUL_COMPLETION,
                format!("DDL: {} {}", obj_type.unwrap_or_default(), identity.unwrap_or_default())
            );
        });
    });
    trigger.new()
}
```

### 1.8 LISTEN/NOTIFY from Rust

PostgreSQL's `LISTEN`/`NOTIFY` provides asynchronous messaging. In pgrx, you can use `pg_sys` or SPI to interact with the notification system.

**Sending notifications from a Rust extension function:**

```rust
use pgrx::prelude::*;
use pgrx::pg_sys;

#[pg_extern]
fn notify_channel(channel: &str, payload: &str) {
    unsafe {
        let channel_c = std::ffi::CString::new(channel).unwrap();
        let payload_c = std::ffi::CString::new(payload).unwrap();
        pg_sys::DirectFunctionCall2(
            pg_sys::pg_notify,
            pg_sys::Datum::from(channel_c.as_ptr()),
            pg_sys::Datum::from(payload_c.as_ptr()),
        );
    }
}
```

**Alternative via SPI:**

```rust
#[pg_extern]
fn notify_via_spi(channel: &str, payload: &str) {
    Spi::connect(|client| {
        let query = format!("SELECT pg_notify('{}', '{}')", channel, payload);
        client.select(&query, None, None).unwrap();
    });
}
```

**Receiving notifications in a background worker:**

While pgrx does not yet provide a high-level `LISTEN` API, a background worker (registered via `pg_sys::RegisterBackgroundWorker`) can maintain a database connection and poll for notifications using PostgreSQL's libpq-compatible functions accessible through `pg_sys`.

```rust
// BackgroundWorker entry point (simplified concept)
#[no_mangle]
#[pg_guard]
pub extern "C" fn background_worker_main(_arg: pg_sys::Datum) {
    unsafe {
        pg_sys::BackgroundWorkerUnblockSignals();
        
        // Connect to database
        let dbname = std::ffi::CString::new("mydb").unwrap();
        let conn = pg_sys::BackgroundWorkerConnection::new(dbname.as_ptr());
        
        // Execute LISTEN
        let listen_cmd = std::ffi::CString::new("LISTEN my_channel").unwrap();
        pg_sys::SPI_connect();
        pg_sys::SPI_exec(listen_cmd.as_ptr(), 0);
        
        // Main loop: poll for notifications
        loop {
            pg_sys::WaitLatch(
                pg_sys::MyLatch,
                pg_sys::WL_LATCH_SET | pg_sys::WL_TIMEOUT | pg_sys::WL_POSTMASTER_DEATH,
                1000, // timeout ms
                pg_sys::PG_WAIT_EXTENSION,
            );
            
            // Check for notifications (pseudo-code)
            // pg_sys::ProcessCompletedNotifies();
            
            if pg_sys::got_sigterm {
                break;
            }
        }
        
        pg_sys::SPI_finish();
    }
}
```

### 1.9 Building a Custom Schema with Tables, Functions, and Types

pgrx extensions can embed custom SQL via `extension_sql!` and `extension_sql_file!` macros, or use the `pg_schema` attribute for tests.

```rust
use pgrx::prelude::*;

pgrx::pg_module_magic!();

// Include custom SQL that creates tables and types
extension_sql!(
    r#"
    CREATE TABLE IF NOT EXISTS my_extension.audit_log (
        id BIGSERIAL PRIMARY KEY,
        event_type TEXT NOT NULL,
        payload JSONB,
        created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    );
    
    CREATE TYPE my_extension.priority AS ENUM ('low', 'medium', 'high');
    "#,
    name = "create_schema",
    bootstrap,
    creates = ["audit_log", "priority"],
);

#[derive(PostgresEnum, Debug)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[pg_extern]
fn log_event(event_type: &str, payload: pgrx::JsonB) -> i64 {
    Spi::connect(|client| {
        let query = format!(
            "INSERT INTO my_extension.audit_log (event_type, payload) VALUES ('{}', '{}') RETURNING id",
            event_type,
            serde_json::to_string(&payload.0).unwrap()
        );
        let result = client.select(&query, None, None).unwrap();
        let id: i64 = result.first().get(1).unwrap().unwrap();
        id
    })
}
```

**Extension control file (`my_extension.control`):**

```
comment = 'My pgrx extension with custom schema'
default_version = '0.0.1'
module_pathname = '$libdir/my_extension'
relocatable = false
schema = my_extension
```

### 1.10 Safety and Memory Management

- **Panic translation:** Rust `panic!` is converted to PostgreSQL `ERROR` (transaction abort, not process crash).
- **Memory contexts:** Use `pgrx::PgMemoryContexts` for safe allocation in PostgreSQL's memory context system.
- **Guard macro:** `#[pg_guard]` wraps `extern "C"` functions passed to PostgreSQL to ensure safe unwinding.
- **Null safety:** `Datum` values are represented as `Option<T>`, with `NULL` safely mapping to `None`.

---

## 2. Ory Keto with PostgreSQL Storage

### 2.1 Overview

Ory Keto is an open-source authorization server implementing the Zanzibar authorization model. It stores relation tuples (namespace, object, relation, subject) and evaluates permissions via graph traversal. PostgreSQL is one of its supported persistence backends.

### 2.2 Tuple Format

The fundamental data structure in Keto is the **relation tuple**, formatted as:

```
namespace:object#relation@subject
```

**Components:**
- **namespace:** A domain or resource type (e.g., `files`, `documents`, `blog_posts`).
- **object:** A specific resource ID (e.g., `folder:projects`, `file:report.pdf`).
- **relation:** The relationship name (e.g., `owner`, `viewer`, `editor`, `parent`).
- **subject:** A user ID or a **subject set** (indirect reference).

**Subject set format:**
```
namespace:object#relation
```

**Examples:**

```
// Direct: user1 is owner of document:doc1
documents:doc1#owner@user1

// Subject set: members of group:admins are editors of document:doc2
documents:doc2#editor@group:admins#member

// Parent-child inheritance: file:report.pdf is a child of folder:projects
files:report.pdf#parent@folder:projects
```

### 2.3 PostgreSQL Database Schema

Keto uses a **per-namespace table strategy** for relation tuples. When namespaces are configured, Keto creates dedicated tables for each namespace.

**Core table naming convention:**
- `keto_{namespace_id}_relation_tuples` — stores relation tuples for a namespace
- `keto_namespace_{namespace_id}_migrations` — tracks schema migrations per namespace

**Example table structure (inferred from logs and Keto internals):**

```sql
-- Table: keto_0000000000_relation_tuples
-- (Keto uses a zero-padded namespace identifier)

CREATE TABLE IF NOT EXISTS keto_0000000000_relation_tuples (
    shard_id         VARCHAR(36) NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    network_id       VARCHAR(36) NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    namespace_id     VARCHAR(36) NOT NULL,
    object           VARCHAR(64) NOT NULL,
    relation         VARCHAR(64) NOT NULL,
    subject_id       VARCHAR(64),
    subject_set_namespace VARCHAR(64),
    subject_set_object    VARCHAR(64),
    subject_set_relation  VARCHAR(64),
    commit_time      TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (shard_id, network_id, namespace_id, object, relation, subject_id, subject_set_namespace, subject_set_object, subject_set_relation)
);

-- Indexes for efficient lookups
CREATE INDEX idx_relation_tuples_subject 
    ON keto_0000000000_relation_tuples(subject_id, subject_set_namespace, subject_set_object, subject_set_relation);

CREATE INDEX idx_relation_tuples_object_relation 
    ON keto_0000000000_relation_tuples(namespace_id, object, relation);

CREATE INDEX idx_relation_tuples_commit_time 
    ON keto_0000000000_relation_tuples(commit_time);
```

**Key observations from Keto logs and source:**
- Keto queries use `relation = $1 AND subject = $2 ORDER BY object, relation, subject, commit_time LIMIT 501 OFFSET 0`.
- Pagination uses **keyset pagination** (page token = last ID from previous page) for stable, fast traversal.
- The `network_id` column supports multi-network isolation (similar to tenant separation).

### 2.4 Namespace Configuration

Namespaces are defined in Keto configuration (YAML) or via the Ory Permission Language (OPL).

```yaml
# keto.yml
namespaces:
  - id: 0
    name: files
  - id: 1
    name: documents

serve:
  read:
    host: 0.0.0.0
    port: 4466
  write:
    host: 0.0.0.0
    port: 4467

dsn: postgres://user:password@localhost:5432/keto?sslmode=disable
```

### 2.5 Reading and Writing Tuples Directly from SQL

**Inserting a relation tuple directly:**

```sql
-- Insert a direct relation tuple
INSERT INTO keto_0000000000_relation_tuples 
    (shard_id, network_id, namespace_id, object, relation, subject_id)
VALUES 
    ('00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000', 
     '0', 'file:report.pdf', 'owner', 'user:alice');

-- Insert a subject set relation tuple
INSERT INTO keto_0000000000_relation_tuples 
    (shard_id, network_id, namespace_id, object, relation, subject_set_namespace, subject_set_object, subject_set_relation)
VALUES 
    ('00000000-0000-0000-0000-000000000000', '00000000-0000-0000-0000-000000000000',
     '0', 'folder:projects', 'viewer', '0', 'group:developers', 'member');
```

**Querying relation tuples:**

```sql
-- Find all objects where user:alice is the owner
SELECT object, relation, commit_time
FROM keto_0000000000_relation_tuples
WHERE namespace_id = '0'
  AND relation = 'owner'
  AND subject_id = 'user:alice'
ORDER BY object, relation, subject_id, commit_time
LIMIT 501;

-- Find all subjects that can view folder:projects
SELECT subject_id, subject_set_namespace, subject_set_object, subject_set_relation
FROM keto_0000000000_relation_tuples
WHERE namespace_id = '0'
  AND object = 'folder:projects'
  AND relation = 'viewer';

-- Check permission (direct match)
SELECT EXISTS (
    SELECT 1 FROM keto_0000000000_relation_tuples
    WHERE namespace_id = '0'
      AND object = 'file:report.pdf'
      AND relation = 'owner'
      AND subject_id = 'user:alice'
);
```

**Time-based permission control (advanced):**

```sql
-- Insert a tuple with a future commit time (effective scheduling)
INSERT INTO keto_0000000000_relation_tuples 
    (namespace_id, object, relation, subject_id, commit_time)
VALUES 
    ('0', 'file:schedule.pdf', 'viewer', 'user:manager', NOW() + INTERVAL '1 hour');
```

### 2.6 Integrating Keto Checks into PostgreSQL Query Context

To enforce Keto authorization at the database layer, you can create a PostgreSQL stored function or extension that queries Keto's PostgreSQL tables (or calls the Keto API).

**Approach 1: Direct SQL check function (co-located database):**

```sql
-- Function to check if a subject has a relation on an object
CREATE OR REPLACE FUNCTION keto_check(
    p_namespace_id TEXT,
    p_object TEXT,
    p_relation TEXT,
    p_subject_id TEXT
)
RETURNS BOOLEAN AS $$
DECLARE
    v_table_name TEXT;
    v_result BOOLEAN;
BEGIN
    -- Keto uses per-namespace tables; determine the correct table
    v_table_name := 'keto_' || LPAD(p_namespace_id, 10, '0') || '_relation_tuples';
    
    -- Execute dynamic query against Keto's tables
    EXECUTE format(
        'SELECT EXISTS (
            SELECT 1 FROM %I 
            WHERE namespace_id = $1 
              AND object = $2 
              AND relation = $3 
              AND (subject_id = $4 OR subject_set_namespace IS NOT NULL)
        )', v_table_name
    ) INTO v_result
    USING p_namespace_id, p_object, p_relation, p_subject_id;
    
    RETURN v_result;
END;
$$ LANGUAGE plpgsql STABLE;

-- Usage in a query with RLS
CREATE POLICY keto_access_policy ON documents
    USING (keto_check('1', 'document:' || id, 'viewer', current_user));
```

**Approach 2: pgrx extension that calls Keto API:**

```rust
use pgrx::prelude::*;
use reqwest::blocking::Client;

#[pg_extern]
fn keto_check_permission(
    namespace: &str,
    object: &str,
    relation: &str,
    subject: &str,
    keto_read_url: &str,
) -> bool {
    let client = Client::new();
    let url = format!(
        "{}/relation-tuples/check?namespace={}&object={}&relation={}&subject_id={}",
        keto_read_url,
        urlencoding::encode(namespace),
        urlencoding::encode(object),
        urlencoding::encode(relation),
        urlencoding::encode(subject)
    );
    
    match client.get(&url).send() {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>() {
                json.get("allowed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        Err(_) => false,
    }
}
```

**Approach 3: Materialized view for fast permission checks:**

```sql
-- Create a flattened permission view
CREATE MATERIALIZED VIEW permission_cache AS
SELECT 
    namespace_id,
    object,
    relation,
    subject_id,
    COALESCE(subject_id, subject_set_namespace || ':' || subject_set_object || '#' || subject_set_relation) AS resolved_subject
FROM keto_0000000000_relation_tuples
WHERE network_id = '00000000-0000-0000-0000-000000000000';

CREATE INDEX idx_permission_cache_lookup ON permission_cache(namespace_id, object, relation, subject_id);

-- Refresh periodically
REFRESH MATERIALIZED VIEW CONCURRENTLY permission_cache;
```

### 2.7 Caching and Performance

Keto supports in-memory caching to reduce database load:

```yaml
# keto.yml
cache:
  enabled: true
  max_size: 100000
  ttl: 5m
```

**Database optimization recommendations:**
- Create composite indexes on `(namespace_id, object, relation, subject_id)`.
- Use connection pooling (e.g., PgBouncer) between Keto and PostgreSQL.
- Regularly clean up expired or deleted relation tuples.
- Use keyset pagination for large result sets (avoids `OFFSET` overhead).

---

## 3. Row-Level Security and Column-Level Encryption in PostgreSQL

### 3.1 Row-Level Security (RLS) Architecture

RLS policies automatically inject `WHERE` clauses into queries based on the current user/session context. This enforces access control at the database layer, independent of application logic.

**Basic RLS setup:**

```sql
-- Enable RLS on a table
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

-- Create a function to get current tenant ID from session context
CREATE OR REPLACE FUNCTION current_tenant_id()
RETURNS INTEGER AS $$
BEGIN
    RETURN current_setting('app.current_tenant_id', TRUE)::INTEGER;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE;

-- Create isolation policy
CREATE POLICY tenant_isolation_policy ON documents
    FOR ALL TO application_role
    USING (tenant_id = current_tenant_id());

-- Force RLS even for table owner (important for defense-in-depth)
ALTER TABLE documents FORCE ROW LEVEL SECURITY;
```

**Setting tenant context in application:**

```sql
-- At the start of each request/transaction
SET app.current_tenant_id = '42';

-- Now all queries on 'documents' are automatically filtered
SELECT * FROM documents; -- effectively adds WHERE tenant_id = 42
```

### 3.2 Multi-Tenant RLS Pattern with Per-Tenant Roles

For stronger isolation, create dedicated database roles per tenant:

```sql
-- Base tenant role
CREATE ROLE tenant NOLOGIN NOINHERIT NOCREATEROLE NOCREATEDB;

-- Per-tenant user
CREATE USER tenant_42 WITH PASSWORD 'secure_password';
GRANT tenant TO tenant_42;

-- Extract tenant ID from role name
CREATE OR REPLACE FUNCTION get_tenant_id_from_role()
RETURNS INTEGER AS $$
BEGIN
    IF current_user ~ '^tenant_[0-9]+$' THEN
        RETURN regexp_replace(current_user, '[^0-9]', '', 'g')::int;
    ELSE
        RETURN NULL;
    END IF;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Policy using role-based tenant ID
CREATE POLICY tenant_role_policy ON users
    USING (tenant_id = get_tenant_id_from_role());
```

### 3.3 Column-Level Encryption with pgcrypto

**Enable extension:**

```sql
CREATE EXTENSION IF NOT EXISTS pgcrypto;
```

**Basic symmetric encryption:**

```sql
-- Encrypt a column value
UPDATE users 
SET ssn = pgp_sym_encrypt(ssn::text, 'my_secure_passphrase');

-- Decrypt on retrieval
SELECT 
    id, 
    pgp_sym_decrypt(ssn, 'my_secure_passphrase') AS ssn_decrypted
FROM users;
```

**AES-256 encryption (deterministic for equality):**

```sql
-- Encrypt with AES-256-CBC (using encrypt function from pgcrypto)
UPDATE users
SET email_encrypted = encrypt(
    email::bytea, 
    '32_byte_key_256_bit_length!!!',  -- 32 bytes for AES-256
    'aes-256-cbc'
);

-- Decrypt
SELECT convert_from(
    decrypt(email_encrypted, '32_byte_key_256_bit_length!!!', 'aes-256-cbc'),
    'UTF8'
) AS email FROM users;
```

**Important note:** `pgp_sym_encrypt` is preferred over raw `encrypt` because it includes integrity checks and handles key derivation properly. For deterministic equality searches, use `pgp_sym_encrypt` with the `cipher-algo=aes256` option and the same IV (only for fields where exact-match queries are needed, accepting the reduced security).

### 3.4 Key Management Patterns

**Pattern 1: Envelope encryption with data encryption keys (DEKs) and key encryption keys (KEKs)**

```sql
-- Table storing encrypted data
CREATE TABLE sensitive_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id INTEGER NOT NULL,
    encrypted_payload BYTEA NOT NULL,
    data_key_id INTEGER NOT NULL,        -- Reference to DEK
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Table tracking DEKs (encrypted by KEK)
CREATE TABLE data_encryption_keys (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL,
    encrypted_dek BYTEA NOT NULL,          -- DEK encrypted by KEK
    kek_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    rotated_at TIMESTAMPTZ
);
```

**Pattern 2: Per-tenant keys with version tracking**

```sql
CREATE TABLE tenant_keys (
    tenant_id INTEGER PRIMARY KEY,
    master_key_version INTEGER NOT NULL DEFAULT 1,
    encrypted_master_key BYTEA NOT NULL,   -- Wrapped by KMS
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    rotated_at TIMESTAMPTZ
);
```

**Pattern 3: Key rotation procedure**

```sql
-- Rotate tenant key while keeping ability to decrypt old data
CREATE OR REPLACE FUNCTION rotate_tenant_key(
    p_tenant_id INTEGER,
    p_new_kms_wrapped_key BYTEA
)
RETURNS VOID AS $$
DECLARE
    v_old_key BYTEA;
    v_new_key BYTEA;
BEGIN
    -- 1. Fetch and unwrap old key from KMS (pseudo-code)
    -- v_old_key := kms_unwrap((SELECT encrypted_master_key FROM tenant_keys WHERE tenant_id = p_tenant_id));
    
    -- 2. Generate new data encryption key
    -- v_new_key := gen_random_bytes(32);
    
    -- 3. Update tenant key record
    UPDATE tenant_keys
    SET master_key_version = master_key_version + 1,
        encrypted_master_key = p_new_kms_wrapped_key,
        rotated_at = CURRENT_TIMESTAMP
    WHERE tenant_id = p_tenant_id;
    
    -- 4. Re-encrypt a batch of data (typically done in background)
    -- UPDATE sensitive_records SET encrypted_payload = reencrypt(encrypted_payload, v_old_key, v_new_key)
    -- WHERE tenant_id = p_tenant_id AND key_version = old_version;
END;
$$ LANGUAGE plpgsql;
```

### 3.5 Integrating with External Key Vaults

**AWS KMS integration pattern:**

```sql
-- Store only the KMS key reference, never the raw key
CREATE TABLE aws_kms_key_refs (
    tenant_id INTEGER PRIMARY KEY,
    kms_key_id TEXT NOT NULL,           -- e.g., "alias/tenant-42-key"
    kms_region TEXT NOT NULL DEFAULT 'us-east-1',
    key_version INTEGER NOT NULL DEFAULT 1
);

-- Encryption function (invokes AWS KMS via external call or pgrx extension)
CREATE OR REPLACE FUNCTION encrypt_with_kms(
    p_plaintext BYTEA,
    p_tenant_id INTEGER
)
RETURNS BYTEA AS $$
DECLARE
    v_kms_key_id TEXT;
BEGIN
    SELECT kms_key_id INTO v_kms_key_id
    FROM aws_kms_key_refs
    WHERE tenant_id = p_tenant_id;
    
    -- Delegated to application layer or pgrx extension:
    -- AWS KMS Encrypt API returns ciphertext blob
    -- For pure SQL, use a foreign data wrapper or PL/Python
    RAISE EXCEPTION 'KMS encryption must be performed via application layer or pgrx extension';
END;
$$ LANGUAGE plpgsql;
```

**HashiCorp Vault integration pattern:**

```sql
-- Store Vault transit key reference
CREATE TABLE vault_key_refs (
    tenant_id INTEGER PRIMARY KEY,
    transit_key_name TEXT NOT NULL,      -- e.g., "tenant-42-data-key"
    vault_path TEXT NOT NULL DEFAULT 'transit/encrypt/',
    key_version INTEGER NOT NULL DEFAULT 1
);
```

**pgrx extension calling HashiCorp Vault Transit:**

```rust
use pgrx::prelude::*;
use reqwest::blocking::Client;
use serde_json::json;

#[pg_extern]
fn vault_encrypt(
    plaintext: Vec<u8>,
    vault_url: &str,
    token: &str,
    transit_key: &str,
) -> Option<Vec<u8>> {
    let client = Client::new();
    let url = format!("{}/v1/transit/encrypt/{}", vault_url, transit_key);
    
    // Base64 encode plaintext for Vault API
    let b64 = base64::encode(&plaintext);
    let body = json!({"plaintext": b64});
    
    let response = client
        .post(&url)
        .header("X-Vault-Token", token)
        .json(&body)
        .send();
    
    match response {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                json.get("data")
                    .and_then(|d| d.get("ciphertext"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.as_bytes().to_vec())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

#[pg_extern]
fn vault_decrypt(
    ciphertext: Vec<u8>,
    vault_url: &str,
    token: &str,
    transit_key: &str,
) -> Option<Vec<u8>> {
    let client = Client::new();
    let url = format!("{}/v1/transit/decrypt/{}", vault_url, transit_key);
    
    let body = json!({"ciphertext": std::str::from_utf8(&ciphertext).unwrap_or("")});
    
    let response = client
        .post(&url)
        .header("X-Vault-Token", token)
        .json(&body)
        .send();
    
    match response {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                json.get("data")
                    .and_then(|d| d.get("plaintext"))
                    .and_then(|p| p.as_str())
                    .and_then(|s| base64::decode(s).ok())
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
```

### 3.6 Combined RLS + Column Encryption Pattern

For a production multi-tenant SaaS with HIPAA/GDPR compliance:

```sql
-- Lab results table with both RLS and column encryption
CREATE TABLE lab_results (
    result_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id TEXT NOT NULL,
    patient_id UUID NOT NULL,
    test_type TEXT NOT NULL,
    result_value BYTEA NOT NULL,           -- Encrypted PHI
    reference_range BYTEA,                 -- Encrypted PHI
    collected_at TIMESTAMPTZ NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1 -- For key rotation tracking
);

-- Enable RLS
ALTER TABLE lab_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE lab_results FORCE ROW LEVEL SECURITY;

-- Tenant isolation policy
CREATE POLICY lab_results_tenant_isolation
ON lab_results
AS RESTRICTIVE
USING (tenant_id = current_setting('app.current_tenant', TRUE));

-- Application role (no BYPASSRLS privilege)
CREATE ROLE app_role NOLOGIN;
GRANT SELECT, INSERT, UPDATE ON lab_results TO app_role;
```

**Application layer encryption/decryption flow:**

1. Application fetches per-tenant AES-256-GCM key from KMS on startup (cached in memory, never persisted).
2. Before `INSERT`: encrypt `result_value` and `reference_range` with tenant key.
3. Set `app.current_tenant` session variable before queries.
4. After `SELECT`: decrypt PHI fields with tenant key.
5. Key rotation: increment `key_version`, re-encrypt data in background batches.

**Stored function for encryption (with pgcrypto as fallback):**

```sql
CREATE OR REPLACE FUNCTION encrypt_phi(
    p_plaintext TEXT,
    p_tenant_key BYTEA
)
RETURNS BYTEA AS $$
BEGIN
    RETURN pgp_sym_encrypt(
        p_plaintext,
        encode(p_tenant_key, 'base64'),
        'cipher-algo=aes256, compress-algo=0'
    )::bytea;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE OR REPLACE FUNCTION decrypt_phi(
    p_ciphertext BYTEA,
    p_tenant_key BYTEA
)
RETURNS TEXT AS $$
BEGIN
    RETURN pgp_sym_decrypt(
        p_ciphertext,
        encode(p_tenant_key, 'base64')
    );
END;
$$ LANGUAGE plpgsql IMMUTABLE;
```

### 3.7 Key Rotation Best Practices

1. **Dual-key period:** Maintain both old and new keys active during rotation.
2. **Batch re-encryption:** Process data in small batches to avoid locking.
3. **Version tracking:** Store `key_version` alongside encrypted data.
4. **Lazy rotation:** Re-encrypt on read/write access rather than full table scan.
5. **KMS integration:** Never store raw KEKs in the database; always fetch from HSM/KMS.
6. **Audit logging:** Log all key access and rotation events via `pg_audit`.

```sql
-- Lazy re-encryption on read (application pattern)
CREATE OR REPLACE FUNCTION read_lab_result(
    p_result_id UUID,
    p_tenant_key BYTEA
)
RETURNS TABLE(result_id UUID, patient_id UUID, test_type TEXT, result_value TEXT) AS $$
DECLARE
    v_record RECORD;
    v_decrypted TEXT;
BEGIN
    SELECT lr.* INTO v_record
    FROM lab_results lr
    WHERE lr.result_id = p_result_id
      AND lr.tenant_id = current_setting('app.current_tenant', TRUE);
    
    IF NOT FOUND THEN
        RETURN;
    END IF;
    
    v_decrypted := decrypt_phi(v_record.result_value, p_tenant_key);
    
    -- If old key version, re-encrypt in background (async)
    IF v_record.key_version < (SELECT MAX(key_version) FROM tenant_keys WHERE tenant_id = v_record.tenant_id) THEN
        PERFORM pg_notify('key_rotation_queue', json_build_object(
            'table', 'lab_results',
            'id', v_record.result_id,
            'tenant_id', v_record.tenant_id
        )::text);
    END IF;
    
    RETURN QUERY SELECT v_record.result_id, v_record.patient_id, v_record.test_type, v_decrypted;
END;
$$ LANGUAGE plpgsql;
```

---

## 4. Architectural Integration Patterns

### 4.1 pgrx + Keto + Encryption Combined Architecture

For a high-security PostgreSQL deployment, these three systems can be integrated:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  Business   │  │  Keto Auth  │  │  Encryption Service │  │
│  │  Logic      │  │  Client     │  │  (KMS/Vault client) │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼────────────────────┼─────────────┘
          │                │                    │
          ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  pgrx Extension: flint_forge                        │     │
│  │  ┌─────────────────────────────────────────────┐    │     │
│  │  │  Custom Functions:                            │    │     │
│  │  │   - keto_check(namespace, obj, rel, subj)   │◄───┼─────┼─── Keto API
│  │  │   - vault_encrypt/decrypt(data, key)        │◄───┼─────┼─── Vault/KMS
│  │  │   - notify_on_change(channel, payload)      │    │     │
│  │  │   - audit_log_ddl()                         │    │     │
│  │  └─────────────────────────────────────────────┘    │     │
│  │  ┌─────────────────────────────────────────────┐    │     │
│  │  │  Custom Schema:                             │    │     │
│  │  │   - audit_log table                         │    │     │
│  │  │   - tenant_keys reference table             │    │     │
│  │  │   - encryption_metadata table               │    │     │
│  │  └─────────────────────────────────────────────┘    │     │
│  └─────────────────────────────────────────────────────┘     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  RLS Policies on Application Tables                      │ │
│  │  ┌─────────┐  ┌──────────┐  ┌────────────────────────┐ │ │
│  │  │ tenants │  │  users   │  │  sensitive_documents   │ │ │
│  │  │  (RLS)  │  │  (RLS)   │  │  (RLS + Encrypted)     │ │ │
│  │  └─────────┘  └──────────┘  └────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Keto PostgreSQL Storage (separate schema or database)   │ │
│  │  ┌─────────────────────────────────────────────────────┐  │ │
│  │  │  keto_0000000000_relation_tuples                    │  │ │
│  │  │  keto_0000000001_relation_tuples  (per namespace)    │  │ │
│  │  └─────────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 pgrx Extension for Unified Security Enforcement

A single pgrx extension can encapsulate all three concerns:

```rust
use pgrx::prelude::*;

pgrx::pg_module_magic!();

// ── Keto integration ──
#[pg_extern]
fn authz_check(
    namespace: &str,
    object: &str,
    relation: &str,
) -> bool {
    let subject = current_user();
    // Call Keto API or query local Keto tables
    true // simplified
}

// ── Encryption helper ──
#[pg_extern]
fn decrypt_column(
    ciphertext: Vec<u8>,
    key_id: i32,
) -> Option<String> {
    // Fetch key from Vault/KMS via HTTP or use SPI to look up
    // cached key in a local table
    Some("decrypted".to_string()) // simplified
}

// ── Event trigger for audit ──
#[pg_trigger]
fn audit_ddl_trigger(trigger: &PgTrigger) -> PgTriggerResult {
    // Log DDL events to audit_log table
    trigger.new()
}

// ── Custom schema SQL ──
extension_sql!(
    r#"
    CREATE SCHEMA IF NOT EXISTS flint_forge;
    
    CREATE TABLE flint_forge.audit_log (
        id BIGSERIAL PRIMARY KEY,
        event_type TEXT NOT NULL,
        object_type TEXT,
        object_identity TEXT,
        username TEXT,
        happened_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    );
    
    CREATE TABLE flint_forge.key_cache (
        key_id SERIAL PRIMARY KEY,
        tenant_id INTEGER NOT NULL,
        key_version INTEGER NOT NULL,
        encrypted_key BYTEA NOT NULL,
        cached_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
    );
    "#,
    name = "bootstrap_schema",
    bootstrap,
);
```

---

## 5. Summary and Recommendations

| Concern | Tool/Pattern | Recommendation |
|---------|------------|----------------|
| Extension development | **pgrx** | Use for compute-heavy, security-critical, or Rust-ecosystem extensions. Prefer PL/pgSQL for simple glue logic. |
| Custom schema | `extension_sql!` / `extension_sql_file!` | Embed DDL in Rust; version with extension. |
| System catalog access | `pgrx::pg_sys` + SPI | Use `pg_sys` for low-level catalog access; SPI for SQL queries. |
| Event triggers | `#[pg_trigger]` + `pg_event_trigger_ddl_commands()` | Log DDL changes; enforce naming conventions. |
| LISTEN/NOTIFY | `pg_notify` via SPI or `pg_sys` | Use for inter-process signaling; background workers for consumers. |
| Authorization storage | **Ory Keto** + PostgreSQL | Use per-namespace tables; query via SQL for co-located checks; API for distributed. |
| Authorization checks | Keto stored function or pgrx extension | Materialize common checks; use Keto API for complex traversal. |
| Row isolation | **RLS** + GUC variables | Always `FORCE ROW LEVEL SECURITY`; set tenant context per transaction. |
| Column encryption | **pgcrypto** + external KMS | Encrypt PHI with AES-256; store keys in KMS/Vault; never in database. |
| Key rotation | Envelope encryption + version tracking | Rotate KEKs via KMS; re-encrypt DEKs in batches; track `key_version` on rows. |
| Audit | **pg_audit** + custom event triggers | Log all DDL, DML, and key access events to tamper-evident store. |

---

## References

- [pgrx GitHub Repository](https://github.com/pgcentralfoundation/pgrx)
- [pgrx Documentation (docs.rs)](https://docs.rs/pgrx/latest/pgrx/)
- [Ory Keto Documentation](https://www.ory.sh/docs/keto)
- [Ory Keto GitHub](https://github.com/ory/keto)
- [Zanzibar Paper (Google)](https://research.google/pubs/pub48190/)
- [PostgreSQL pgcrypto Documentation](https://www.postgresql.org/docs/current/pgcrypto.html)
- [PostgreSQL Row-Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [PostgreSQL Event Triggers](https://www.postgresql.org/docs/current/event-triggers.html)
- [PostgreSQL LISTEN/NOTIFY](https://www.postgresql.org/docs/current/sql-notify.html)
- [HashiCorp Vault Transit Engine](https://developer.hashicorp.com/vault/docs/secrets/transit)
- [AWS KMS Developer Guide](https://docs.aws.amazon.com/kms/latest/developerguide/)

