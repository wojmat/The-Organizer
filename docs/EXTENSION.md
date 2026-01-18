# Browser Extension Integration (Chromium)

The Organizer exposes a local-only HTTP bridge so a Chromium extension can fetch matching entries and autofill credentials while your vault is unlocked.

## Setup

1. In the dashboard, enable **Browser extension** and copy the endpoint + token.
2. In Chromium, open `chrome://extensions` and enable **Developer mode**.
3. Click **Load unpacked** and select the `browser-extension` folder from this repo.
4. Open the extension popup, paste the endpoint and token, then save.
5. Navigate to a login page and select an entry to autofill.

## Local Bridge API

All requests require the `X-Organizer-Token` header. The bridge only listens on `127.0.0.1`.

- `GET /v1/status` -> `{ "locked": boolean }`
- `GET /v1/entries?url=<page url>` -> `{ "entries": [{ id, title, username, url }] }`
- `GET /v1/secret?id=<entry id>` -> `{ "password": string }`

If the vault is locked, the bridge returns HTTP 423 with `{ "error": "vault is locked" }`.

## Security Notes

- The bridge never unlocks the vault; it only works while the desktop app is already unlocked.
- Rotate the token from the dashboard to revoke extension access.
- The token is stored locally on the desktop and in the browser extension.
