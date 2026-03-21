## Core Service

Design token pipeline that transforms and distributes tokens across platforms.

## Actors

### Primary

| Name | Description |
| --- | --- |
| Frontend Developer | Consumes tokens in React/Vue apps |
| Designer | Creates and updates tokens in Figma |

### Secondary

| Name | Description |
| --- | --- |
| Design System Lead | Governs token naming and structure |

### Tertiary

| Name | Description |
| --- | --- |
| CI/CD Pipeline | Automates token builds on merge |

## Channels

| Channel | Type | Description |
| --- | --- | --- |
| CLI | Direct | Developer runs commands locally |
| Figma Plugin | Integration | Syncs tokens from Figma |
| GitHub Action | Automated | Builds on push to main |

## Value Exchanges

| Actor | Gives | Gets |
| --- | --- | --- |
| Designer | Token definitions | Consistent implementations |
| Developer | Implementation feedback | Ready-to-use tokens |

## Failure Modes

| Mode | Impact | Recovery |
| --- | --- | --- |
| Token sync fails silently | Stale values in production | Manual re-sync via CLI |
| Schema validation error | Build breaks | Fix token file, re-run |
