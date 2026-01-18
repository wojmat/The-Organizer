# UX behavior notes

Search:
- The search box filters entries locally in the UI (no backend call).
- It matches across: title, username, URL, notes.
- If there are no entries, search is disabled and shows a hint placeholder.

Notes visibility:
- Notes are stored and returned from the backend in `EntryPublic`.
- Notes are shown in two ways:
  - Desktop: a "Notes" badge appears on rows with notes; hovering shows a tooltip with the full note.
  - All platforms: click an entry row to expand details and view notes and URL.

Reason:
- Keeping the list compact avoids leaking long notes on screen while still making them available quickly.

Sorting and activity:
- Entries can be sorted by most used (based on session interactions), recent updates, recent creation, or title.
- The session activity panel shows recent actions for the current app run only.
