# Retrieving and validating secrets against Azure Key Vault

## Quick Setup

You need to have your own Azure Key Vault. For obvious reasons, the project doesn't come pre-configured to connect to any specific Azure Key Vault. But we can explain how to test if this will work for you. This [longer article](https://learn.microsoft.com/en-us/azure/key-vault/secrets/quick-create-cli) might help if you want an explainer first. 

You'll need the `az` command installed to follow this guide (`dnf install azure-cli` if you're on a fedora-based distribution).

1. **Login** with `az login`.
2. **Create a Key Vault.** I did this through the portal. Looks like an equivalent command might be:
```powershell
az keyvault create --name "<your-unique-keyvault-name>" --resource-group "myResourceGroup"
``` 
3. **Give your account permission to manage secrets.**
```powershell
az role assignment create --role "Key Vault Secrets Officer" --assignee "user@domain.com" --scope "/subscriptions/<36-char subscription ID>/resourceGroups/<resource group>/providers/Microsoft.KeyVault/vaults/<your-unique-keyvault-name>"
```
- Omit the &lt; and &gt; signs when you substitute in your own values.
- `user@domain.com` is a user in your Azure instance with appropriate permissions.
- That 36-char sub id can be validated [here](https://portal.azure.com/#view/Microsoft_Azure_Billing/SubscriptionsBladeV2). It looks like its a UUID/GUID format, usually.
- &lt;your-unique-keyvault-name&gt; is whatever you named the Key Vault in the prior step, for example my-unq-kv-2025-04-01.
- &lt;resource group&gt; is the name of the resource group you created the key vault within, for example myResourceGroup.

4. **Post your first secret.**
Name your first secret whatever you want. This will upload the contents of the file to the secret.
```powershell
az keyvault secret set --vault-name "your-unique-keyvault-name" --name "secret-name-whatever-you-choose" --file /path/to/file
```

5. **Create an input json for this program.** `podman-compose-mgr` uses json file to validate that each on-disk secret matches values in Azure Key Vault. The json input contains no sensitive information; it is just a record of files in the key vault, so that subsequent runs of `podman-compose-mgr` can determine if something on the host (or in the key vault) differs.

You can manually build the one-time json input file according to this specification, or you can use the script below to automate it for this simple example. 

JSON specification:
```json
[
  {
    /* path to the file on your disk */
    "filenm": "/path/to/file",

    /* md5 of the file's contents on disk */
    "md5": "eac66e80e21d414126fc47c49f0afeb1",

     /* date you built this json, not actually used by podman-compose-mgr */
    "ins_ts": "2025-03-30T15:14:01,612552867-05:00",

    /* .id attr if you use `az keyvault secret show` */
    "az_id": "https://your-unique-keyvault-name.vault.azure.net/secrets/secret/rest_of_the_url",

     /* .created attr if you use `az keyvault secret show` */
    "az_create": "2025-03-30T20:03:43+00:00",

    /* .updated attr if you use `az keyvault secret show` */
    "az_updated": "2025-03-30T20:03:43+00:00",

    /* .name attr if you use `az keyvault secret show` */
    "az_name": "secret-name-whatever-you-choose",
    
    /* used by podman-compose-mgr when validating on-disk secrets; only those entries in here where hostname matches running hostname are compared to Azure Key Vault */
    "hostname": "your_computer_hostname"
  }
]
```

You can use this script to create the json. This script needs need `jq` installed; `sed`, `awk`, `hostname`, `date`, and `md5sum`/`md5` are also required but typically available.
```shell
#!/usr/bin/env sh

# Set your variables here, you must type your own values in
VAULT_NAME="your-unique-keyvault-name"
SECRET_NAME="secret-name-whatever-you-choose"
FILENAME="/path/to/file"
HOSTNAME=$(hostname)

# Get MD5
if command -v md5sum >/dev/null 2>&1; then
  MD5=$(md5sum "$FILENAME" | awk '{print $1}')
elif command -v md5 >/dev/null 2>&1; then
  MD5=$(md5 -q "$FILENAME")
else
  echo "Error: No md5sum or md5 command found." >&2
  exit 1
fi

# Detect nanosecond support in `date`
if date +"%N" | grep -q 'N'; then
  # date without nanoseconds
  INS_TS=$(date +"%Y-%m-%dT%H:%M:%S%z")
  # Add fake fractional second to match desired format
  INS_TS="${INS_TS:0:19}.000000000${INS_TS:19}"
else
  # Linux with nanosecond support
  INS_TS=$(date +"%Y-%m-%dT%H:%M:%S.%N%z")
fi

# Insert colon in timezone offset to match ISO 8601
INS_TS=$(echo "$INS_TS" | sed -E 's/([+-][0-9]{2})([0-9]{2})$/\1:\2/')

# Get secret info from Azure
SECRET_JSON=$(az keyvault secret show --name "$SECRET_NAME" --vault-name "$VAULT_NAME" --output json)

# Parse required fields
AZ_ID=$(echo "$SECRET_JSON" | jq -r '.id')
AZ_CREATE=$(echo "$SECRET_JSON" | jq -r '.attributes.created')
AZ_UPDATED=$(echo "$SECRET_JSON" | jq -r '.attributes.updated')
AZ_NAME=$(echo "$SECRET_JSON" | jq -r '.name')

# Build the final JSON
jq -n \
  --arg filenm "$FILENAME" \
  --arg md5 "$MD5" \
  --arg ins_ts "$INS_TS" \
  --arg az_id "$AZ_ID" \
  --arg az_create "$AZ_CREATE" \
  --arg az_updated "$AZ_UPDATED" \
  --arg az_name "$AZ_NAME" \
  --arg hostname "$HOSTNAME" \
  '[{
    filenm: $filenm,
    md5: $md5,
    ins_ts: $ins_ts,
    az_id: $az_id,
    az_create: $az_create,
    az_updated: $az_updated,
    az_name: $az_name,
    hostname: $hostname
  }]'
```

6. 