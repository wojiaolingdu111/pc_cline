---
name: license-system-extension
description: How to add new license attributes across the full-stack Tauri license system (server, client Rust, client TS, Vue UI, admin panel)
source: auto-skill
extracted_at: '2026-06-04T02:14:41.831Z'
---

# License System Extension Pattern

When adding new license attributes (like `is_root`, `tier`, `features`), follow this coordinated multi-layer approach:

## Architecture Overview

```
Server (Rust/Axum + SQLite)
    ↓ HTTP API (verify/activate)
Client Rust (Tauri commands + LicenseManager)
    ↓ invoke()
Client TypeScript API (LicenseInfo type)
    ↓ store/reactive
Vue UI (badge display, modals)
Admin Panel (HTML/Vue management UI)
```

## Implementation Checklist

### 1. Server-Side (docs/api/src/main.rs)

**Database Schema:**
- Add column to `licenses` table in `init_db()`:
  ```rust
  conn.execute_batch(
      "CREATE TABLE IF NOT EXISTS licenses (
          ...,
          is_root INTEGER DEFAULT 0
      )",
  )?;
  // Backward compatibility for existing DBs
  conn.execute_batch(
      "ALTER TABLE licenses ADD COLUMN is_root INTEGER DEFAULT 0",
  ).ok();
  ```

**Query Helper:**
- Update `query_license()` SELECT and JSON output to include new field
- Use `COALESCE(column, default)` for NULL safety

**Handler Updates:**
- `handle_add`: Accept new field in request body, insert into DB
- `handle_activate`: Check new field to modify validation logic (e.g., skip expiration)
- `handle_verify`: Return new field in response
- All admin handlers (`handle_list`, etc.): Include new field in responses

**Example - Root License Logic:**
```rust
let is_root = record["is_root"].as_bool().unwrap_or(false);
if !is_root {
    // Check expiration only for non-root
    if let Some(exp) = record["expires_at"].as_i64() {
        if now_ms() > exp {
            return Ok(Json(serde_json::json!({"valid": false, "message": "授权码已过期"})));
        }
    }
}
```

### 2. Client Rust Layer (src-tauri/src/license.rs + commands.rs)

**LicenseStore (persisted to license.json):**
```rust
#[derive(Debug, Serialize, Deserialize)]
struct LicenseStore {
    trial_start_ms: u64,
    license_key: Option<String>,
    machine_id: String,
    #[serde(default)]
    is_root: bool,  // New field with default
}
```

**LicenseInfo (sent to frontend):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub status: LicenseStatus,
    pub trial_days_total: u32,
    pub trial_days_left: i64,
    pub license_key: Option<String>,
    pub is_root: bool,  // New field
    pub message: String,
}
```

**LicenseManager.get_info():**
- Check new field and return appropriate status/message:
  ```rust
  if store.is_root {
      return LicenseInfo {
          status: LicenseStatus::Active,
          is_root: true,
          message: "永久授权".to_string(),
          ...
      };
  }
  ```

**LicenseManager.set_license_key():**
- Accept new field as parameter, persist to store:
  ```rust
  pub fn set_license_key(&mut self, key: String, is_root: bool) -> Result<()>
  ```

**Tauri Command (commands.rs):**
- Extract new field from server response, pass to manager:
  ```rust
  let is_root = result.get("is_root").and_then(|v| v.as_bool()).unwrap_or(false);
  state.license.lock().unwrap().set_license_key(key, is_root)?;
  ```

### 3. Client TypeScript API (src/api/tauri.ts)

**LicenseInfo Interface:**
```typescript
export interface LicenseInfo {
    status: "Trial" | "Active" | "Expired";
    trial_days_total: number;
    trial_days_left: number;
    license_key: string | null;
    is_root: boolean;  // New field
    message: string;
}
```

**Browser Fallback:**
- Add new field with default value to mock responses:
  ```typescript
  return {
      status: "Trial",
      ...,
      is_root: false,
      message: "浏览器预览模式",
  };
  ```

### 4. Vue UI (src/App.vue)

**Computed Properties:**
- Add conditional logic for new field:
  ```typescript
  const licenseBadge = computed(() => {
      if (licenseInfo.value?.is_root) return "永久授权";
      if (licenseInfo.value?.status === "Active") return "已激活";
      ...
  });
  
  const licenseBadgeClass = computed(() => {
      if (licenseInfo.value?.is_root) return "license-root";
      ...
  });
  ```

**CSS Styling:**
- Add distinct visual treatment:
  ```css
  .license-root {
      background: linear-gradient(135deg, #7c3aed, #a855f7);
  }
  ```

### 5. Admin Panel (docs/api/admin/index.html)

**Add Modal:**
- Add checkbox/input for new field:
  ```html
  <label style="display:flex;align-items:center;gap:8px">
      <input type="checkbox" v-model="newIsRoot" />
      <span>Root 授权（永久有效）</span>
  </label>
  ```

**Vue Data:**
```javascript
data() {
    return {
        ...,
        newIsRoot: false,
    };
}
```

**API Call:**
- Send new field in POST body:
  ```javascript
  body: JSON.stringify({
      code,
      max_activations: this.newMax,
      is_root: this.newIsRoot || undefined,
  })
  ```

**Table Display:**
- Update status helpers:
  ```javascript
  statusText(k) {
      if (k.is_root) return "Root 永久";
      ...
  }
  ```

- Add conditional UI (hide irrelevant actions):
  ```html
  <button v-if="!k.is_root" @click="setExpire(k.code)">设过期</button>
  ```

**Styling:**
- Add badge/dot styles:
  ```css
  .badge-root { background: linear-gradient(135deg, #ede9fe, #f3e8ff); color: #6d28d9; }
  .badge-dot.purple { background: #7c3aed; }
  ```

## Key Principles

1. **Coordinated Changes**: All 5 layers must be updated together - missing one breaks the flow
2. **Backward Compatibility**: Use `#[serde(default)]` and `COALESCE()` for existing data
3. **Type Safety**: Match field names exactly across Rust/TypeScript (camelCase in TS, snake_case in Rust with `#[serde(rename_all = "camelCase")]`)
4. **Validation Logic**: New attributes often change validation rules (e.g., root skips expiration)
5. **Visual Distinction**: Use distinct colors/styles for special license types
6. **Admin UX**: Hide irrelevant actions (e.g., can't set expiration on permanent licenses)

## Testing Flow

1. Create license via admin panel with new attribute
2. Verify server returns correct field in API responses
3. Activate in desktop app - check persisted `license.json`
4. Verify UI badge displays correctly
5. Restart app - ensure persistence works
