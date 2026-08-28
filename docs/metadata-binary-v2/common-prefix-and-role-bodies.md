# 3. Common body prefix

Every role body starts with:

| Offset | Size | Field |
| ---: | ---: | --- |
| `0x00` | 8 | Role version, little-endian |
| `0x08` | 8 | Expiry Unix time, zero means no trusted clock |

The role-specific body follows immediately. Canonical encoders emit fields in
the order defined below and reject trailing bytes.

## 4. Role bodies

Fixed binary values use their natural width. Text uses `u16 length` followed
by UTF-8 bytes. The decoder validates UTF-8, the declared bound, and the role
contract before exposing a borrowed slice.

Root body:

```text
common prefix
u8 key_count
key_count * (key_id[16], role:u8, public_key[32])
u8 role_count
role_count * (role:u8, threshold:u8, key_count:u8, key_ids[key_count * 16])
```

Timestamp body:

```text
common prefix
snapshot_version:u64
snapshot_length:u32
snapshot_sha256:[u8;32]
```

Snapshot body:

```text
common prefix
targets_reference(version:u64, length:u32, sha256:[u8;32])
revocations_reference(version:u64, length:u32, sha256:[u8;32])
u8 delegation_count
delegation_count * (id:text, version:u64, length:u32, sha256:[u8;32])
```

Targets body:

```text
common prefix
u16 delegation_count
delegation_count * delegation_id:text
u32 package_count
package_count * (record_length:u16, target_record)
```

Each target record retains the existing semantic fields, encoded in fixed
order. A record length lets the kernel skip non-selected records without
retaining them. The F405 loader parses and authorizes one selected record at a
time.

Delegation body:

```text
common prefix
developer_id:text
key_id:[u8;16]
public_key:[u8;32]
u8 namespace_count, namespace_count * namespace:text
u8 target_count, target_count * target_profile:text
u8 abi_count, abi_count * abi:u16
not_before:u64
not_after:u64
```

Revocation body:

```text
common prefix
u16 record_count
record_count * (key_id:[u8;16], revoked_at:u64, reason:text)
```
