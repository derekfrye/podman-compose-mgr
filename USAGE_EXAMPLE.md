# Usage Example for R2 Storage

## Setting up credentials for R2 Storage

To use Cloudflare R2 storage, you need to add the following command line arguments:

```
--r2-account-id YOUR_CLOUDFLARE_ACCOUNT_ID \
--r2-access-key-id-filepath /path/to/r2_access_key_id.txt \
--r2-access-key-filepath /path/to/r2_access_key.txt \
--r2-bucket-for-upload YOUR_BUCKET_NAME_OR_FILEPATH
```

## Example of input.json for R2 storage

```json
[
  {
    "filenm": "/path/to/your/file.txt",
    "md5": "file_hash_here",
    "ins_ts": "2024-10-12T20:36:26.612751504-05:00",
    "destination_cloud": "r2",
    "hostname": "your_hostname",
    "encoding": "utf8",
    "file_size": 1024,
    "encoded_size": 1024
  }
]
```

The key field is `"destination_cloud": "r2"` which directs the file to be uploaded to Cloudflare R2 storage rather than Azure KeyVault or Backblaze B2.

## Full command example

```bash
cargo run -- --mode secret-upload \
  --verbose \
  --secrets-client-id /path/to/client_id.txt \
  --secrets-client-secret-path /path/to/secret.txt \
  --secrets-tenant-id /path/to/tenant_id.txt \
  --secrets-vault-name /path/to/vault_name.txt \
  --output-json /path/to/output.json \
  --input-json /path/to/input.json \
  --r2-account-id YOUR_CLOUDFLARE_ACCOUNT_ID \
  --r2-access-key-id-filepath /path/to/r2_access_key_id.txt \
  --r2-access-key-filepath /path/to/r2_access_key.txt \
  --r2-bucket-for-upload YOUR_BUCKET_NAME
```