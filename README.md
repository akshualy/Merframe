# Merframe

![Merframe header image](.github/Merframe.png)

Merframe is an open-source alternative to AlecaFrame that runs natively on Linux and Windows.

Merframe displays information about:
 - World timers, fissures, alerts, invasions and events
 - Arbitrations
 - Foundry
 - Inventory
 - Relics
 - Rivens
 - Mastery
 - Resources
 - warframe.market trading
 - Trading Analytics
 - Stats

In-game overlays show relic rewards, relic recommendations, riven grades, live Arbitration stats, in-game notifications and Ducatering (selling Prime parts at the Ducat Kiosk). They are available on Windows, X11 and XWayland (This includes Wayland and Hyprland since Proton uses XWayland). For more details, read the [Overlays Wiki page](https://github.com/akshualy/Merframe/wiki/Overlays).

Notifications for fissures, rewards, Arbitrations, cycle timers, trades and new conversations are available as normal desktop notifications, in-game pop-ups, and a Discord webhook. For more info, check the [Notifications Wiki page](https://github.com/akshualy/Merframe/wiki/Notifications).

You can create scripts or allow other programs on your PC to listen to Merframe events through the [Event Feed](https://github.com/akshualy/Merframe/wiki/Event-Feed-for-other-applications). This is off by default.

Big Warframe script updates (usually big content drops) will break the overlays until Merframe is updated as well. Changes in data structures from content we get from Warframe Community Developers (WFCD) may break the app. Please create an issue if either of these go unnoticed for a day.

You can download the latest release on the [releases page](https://github.com/akshualy/Merframe/releases), or on the [official Merframe website](https://yareli.net/merframe).

Bug reports, feature requests, and contributions are welcome.

![Preview of the Merframe UI](.github/Preview.png)

## Contents

- [Notice](#notice)
- [Running the app](#running-the-app)
- [Running the app from source](#running-the-app-from-source)
- [Privacy](#privacy)
- [How does Merframe work?](#how-does-merframe-work)
- [On memory (Linux)](#on-memory-linux)
- [Credits](#credits)
- [License](#license)

## Notice

Merframe is a fan project and is not affiliated with Digital Extremes.
Warframe and the game artwork under `app/public/game/` are the property of Digital Extremes Ltd. and are used under its content policy for non-commercial fan projects.

For legality and risks of the app, visit our [Legality & Risks Wiki page](https://github.com/akshualy/Merframe/wiki/Legality-&-Risks).

## Running the app

Releases are provided as an `.exe` or `.msi` installer (Windows), `.deb`, `.rpm`, Arch `.pkg.tar.zst` packages, and an `AppImage`.

Windows may warn you on install that the application is not known. This is normal, and I'm not willing to pay Microsoft to make it go away.

`AppImage` is the preferred way of running the application on Linux.
Due to how GitHub serves files, you must set the execution permission yourself: `chmod +x ./Merframe_amd64.AppImage`.

The app will offer you a self-installing update when one is available. You may disable this in settings or at any update offer.

Package details, the Arch package and how updates work are on the [Installing and Updating Wiki page](https://github.com/akshualy/Merframe/wiki/Installing-and-Updating).

## Running the app from source

Rust workspace under `crates/`, Tauri 2 app with a React front end under `app/`.

You will need:
- `rustup` [(link to rustup)](https://rustup.rs/)
- `pnpm` package manager [(link to pnpm)](https://pnpm.io/installation)

```bash
cd app
pnpm install  # Only on first run or after updates
pnpm tauri dev
```

You can also build a debug binary and run that instead:
```bash
cd app
pnpm tauri build --debug
../target/debug/merframe  # Linux, available after the above command
..\target\debug\merframe.exe  # Windows
```

Running the app from cargo is not supported as that will not start the Vite server.

When a Rust type sent to the front end changes, run `pnpm types` in `app/` to regenerate `app/src/types/generated/` and its `index.ts`.
`pnpm types:check` fails when this needs to be done. The generated files are committed so that `pnpm build` doesn't need cargo.

Diagnostics: `RUST_LOG=merframe_lib=debug,wf_scan=debug pnpm tauri dev`.
App data: `~/.local/share/Merframe/` on Linux, `%APPDATA%\Merframe\` on Windows.
App data holds the SQLite store, the last inventory as `inventory.json.gz`, WFCD image/data cache, settings, and `merframe.log`.

## Privacy

The warframe.market session token (allows acting as you) is preferably stored in Windows Credential Manager or the desktop Secret Service on Linux (for example GNOME Keyring or KDE Wallet). If secure storage is unavailable or cannot retain the token, Merframe saves it in plaintext in `market_token` in the app data directory. On Linux this file is restricted to the current user. Login still works without a credential store.

Plaintext sessions migrate into secure storage on the next session load or save when it is available; their files and legacy JSON copies are removed only after the secure write is verified. A fallback file takes precedence over an older secure session. Logout remains effective while secure storage is unavailable, and deletion of the secure copy is retried on the next startup. If neither storage option can save the login or logout, Merframe reports an error. Other programs running as your user may be able to access either kind of stored session. The token does not contain your password.

Inventory and analytics are kept locally. Signing in sends your email and password directly to warframe.market over HTTPS, and authenticated trading requests send the session token to that service. Optional Discord notifications send configured alert text to your webhook. Public item/price lookups also expose your IP address to their providers.

Services Merframe talks to:
- `api.warframe.com`: the public world state, polled every 5 to 10 minutes.
- `api.warframe.market`: orders and riven auctions. Prices work without a login, only listing items needs one. While you are logged in, your unread message count is retrieved alongside your orders.
- `browse.wf`: the Arbitration forecast and the community tiers, fetched every 6 hours.
- `api.merframe.net`: a price cache run by me, so that Merframe does not ask warframe.market for every item separately. The app fetches this every 15 minutes by default and only downloads it when it changed. The cache itself checks Prime parts and sets every 30 minutes, other items with orders every 4 hours, the rest daily, and refreshes trade volumes and the 90-day history once a day. A price in Merframe is for Prime items at most about half an hour old, for anything else likely a couple hours.
- `raw.githubusercontent.com`: item data from WFCD's `warframe-items`.
- `cdn.warframestat.us`: item images, downloaded once and kept in the app data directory.
- A Discord webhook, only if you set one up under notifications. It receives the alert types you pick there.

## How does Merframe work?

Merframe reads out what the game stores in its own space on your computer - the game's memory - and reads the game's log file (`EE.log`).
Reading memory is the same approach AlecaFrame uses (via Overwolf) for inventory data.

Merframe goes a bit further and takes this approach for all data: Your current inventory, displayed riven, riven rolls, relic fissure & relic rewards, trades and the Ducat Kiosk. AlecaFrame uses OCR for most of this, which is arguably less risky.

DE has so far tolerated memory reading, but they can not give a clear okay or recommendation for third-party tools.
What that can mean for your account is on the [Legality & Risks Wiki page](https://github.com/akshualy/Merframe/wiki/Legality-&-Risks).
As with all user tools: Use at your own risk.

Memory holds way more than the game shows to a user. Merframe strives to only display what a player could also work out by hand from what the game shows them. To put it in a concrete rule: If it can be done with pen and paper, Merframe may show it.

## On memory (Linux)

Merframe reads the memory of the running game through `/proc/<pid>/mem`.
With `kernel.yama.ptrace_scope` at 0 or 1, the default of most distributions, this works without any setup.
If Merframe has an error at the tope saying `Reading process <pid> is not permitted`, your system is at scope 2 or 3. The [Memory permissions on Linux Wiki page](https://github.com/akshualy/Merframe/wiki/Memory-permissions-on-Linux) has more detail on what to do in these cases.

## Credits

Merframe stands on the work of others. Thank you to:
- [AlecaFrame](https://alecaframe.com/), whose feature set Merframe re-creates for Linux and Windows. No AlecaFrame code is used.
- [warframe.market](https://warframe.market) for the trading platform and its public API.
- The [Warframe Community Developers](https://github.com/WFCD): item data from [`warframe-items`](https://github.com/WFCD/warframe-items), the node, sortie and Nightwave tables from [`warframe-worldstate-data`](https://github.com/WFCD/warframe-worldstate-data), the world cycle arithmetic documented by [`warframe-worldstate-parser`](https://github.com/WFCD/warframe-worldstate-parser), and the item images served from `cdn.warframestat.us`.
- The editors of the [Warframe wiki](https://wiki.warframe.com), the source of the Helminth ability table and of the items the exports do not carry.
- Digital Extremes for Warframe, its public worldState and a content policy that lets fan projects exist.
- [ChickenDrawsDogs](https://x.com/ChickenDraws) for their awesome `Dog Days "Hang Loose" Poster` drawing, which is the header image.
- [Tauri](https://tauri.app), [React](https://react.dev), [shadcn/ui](https://ui.shadcn.com), [Radix UI](https://www.radix-ui.com), [Tailwind CSS](https://tailwindcss.com) and [Lucide](https://lucide.dev), which the app is built with.

## License

Merframe is free software under the GNU General Public License, version 3 or (at your option) any later version. The full text is in [`LICENSE`](LICENSE). It comes without any warranty.
The game artwork is excluded, see [Notice](#notice).
