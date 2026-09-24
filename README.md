# Star Drifter

Procedural space-drifting arcade (vanilla JS + canvas, zero deps, no build step).
Live at https://stardrifter.cosmicrafts.com — embedded in the Cosmicrafts game library.

The legacy Rust/Bevy version is frozen on the `legacy-bevy` branch.

## Play locally

```bash
cd public && python3 -m http.server 8000
# open http://127.0.0.1:8000/
```

## How it works

- `public/index.html` — shell: faction menu, game-over overlay, canvas
- `public/game.js` — the whole game: seeded sector generation, inertia/drift
  physics, drift-chain scoring, closing Dark Rift storm, warp gates
- `public/style.css` — menus and overlays
- `public/assets/icons/` — sector iconography (kept from the Bevy version)

Game design (factions, the 10 sector types, Aetherium/Hull) follows the
original Bevy design doc; the arcade drift gameplay delivers what the site
promises: drift between sectors, chain your drift, survive the Dark Rift.

## Deploy

Push to `main` → GitHub Actions rsyncs `public/` to an immutable release dir
on Ionos, flips `/var/www/stardrifter.cosmicrafts.com/current` atomically,
syncs `infra/nginx/stardrifter.cosmicrafts.com.conf` (backup + `nginx -t` +
reload) and verifies live.
